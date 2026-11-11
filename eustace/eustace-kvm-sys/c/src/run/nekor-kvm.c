#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <linux/kvm.h>

#include "nekor-kvm-miscenalleous.h"
#include "nekor-kvm.h"

/*
 * Check mapped KVM run images and access their active exit records and variable-length data.
 */
NEKOR_KVM_STATIC_ASSERT(NEKOR_KVM_SIZEOF(nekor_kvm_storage_size_t) == NEKOR_KVM_SIZEOF(size_t),
                        "The Nekor storage extent must match the compiler size type.");
NEKOR_KVM_STATIC_ASSERT(NEKOR_KVM_SIZEOF(nekor_kvm_descriptor_t) == NEKOR_KVM_SIZEOF(int),
                        "The Nekor descriptor type must match the host descriptor type.");

/*
 * Keep the native representation translation on the pinned KVM API baseline. A header update
 * that changes this version requires an explicit review of the bridge interface before it can
 * compile again.
 */
NEKOR_KVM_STATIC_ASSERT(KVM_API_VERSION == 12, "The C translation layer requires KVM API version "
                                               "12.");

/**
 * Validate the fixed run-image extent and its native KVM alignment.
 *
 * The pointer is interpreted as `struct kvm_run` only after it covers that fixed representation
 * and satisfies the alignment required by the selected Linux headers. The variable port-I/O data
 * extent is validated separately by `nekor_kvm_run_io_extent`.
 */
static NEKOR_KVM_FORCEINLINE nekor_kvm_status_t
nekor_kvm_run_validate(const void *run_image, nekor_kvm_storage_size_t run_size)
{
    if (NEKOR_KVM_UNLIKELY(!run_image))
        return -EINVAL;

    if (NEKOR_KVM_UNLIKELY(run_size < NEKOR_KVM_SIZEOF(struct kvm_run)))
        return -ENOBUFS;

    if (NEKOR_KVM_UNLIKELY((uintptr_t)run_image % NEKOR_KVM_ALIGNOF(struct kvm_run)))
        return -EINVAL;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Validate and measure the flexible data range of a port-I/O exit.
 *
 * KVM stores this range at `data_offset` from the beginning of the run mapping. This helper checks
 * the active exit tag, direction, element width, conversion to `size_t`, multiplication, and
 * final mapped extent before any caller copies bytes from or into the range.
 */
static nekor_kvm_status_t nekor_kvm_run_io_extent(const struct kvm_run *run,
                                                  nekor_kvm_storage_size_t run_size,
                                                  nekor_kvm_storage_size_t *data_offset,
                                                  nekor_kvm_storage_size_t *data_size)
{
    const uint64_t target_data_offset = run->io.data_offset;
    const uint8_t target_element_size = run->io.size;
    const uint32_t target_element_count = run->io.count;
    size_t target_data_size;

    if (NEKOR_KVM_UNLIKELY(run->exit_reason != KVM_EXIT_IO))
        return -EPROTO;

    if (NEKOR_KVM_UNLIKELY(run->io.direction != KVM_EXIT_IO_IN &&
                           run->io.direction != KVM_EXIT_IO_OUT))
        return -EBADMSG;

    if (NEKOR_KVM_UNLIKELY(target_element_size != NEKOR_KVM_IO_ELEMENT_SIZE_BYTE &&
                           target_element_size != NEKOR_KVM_IO_ELEMENT_SIZE_WORD &&
                           target_element_size != NEKOR_KVM_IO_ELEMENT_SIZE_DWORD))
        return -EBADMSG;

    if (NEKOR_KVM_UNLIKELY((uintmax_t)target_data_offset > (uintmax_t)SIZE_MAX))
        return -ERANGE;

    if (NEKOR_KVM_UNLIKELY((uintmax_t)target_element_count >
                           (uintmax_t)(SIZE_MAX / target_element_size)))
        return -EOVERFLOW;

    target_data_size = (size_t)target_element_count * target_element_size;
    *data_offset = (size_t)target_data_offset;
    *data_size = target_data_size;

    if (NEKOR_KVM_UNLIKELY(*data_offset > run_size || target_data_size > run_size - *data_offset))
        return -ERANGE;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Validate and classify the active MMIO arm.
 *
 * The run structure's MMIO union member is meaningful only for `KVM_EXIT_MMIO`. Its length must
 * fit the eight-byte data member, and `is_write` must use one of the two values defined by the
 * UAPI before it is translated into the Nekor direction enum.
 */
static NEKOR_KVM_INLINE nekor_kvm_status_t
nekor_kvm_run_mmio_direction(const struct kvm_run *run, nekor_kvm_mmio_direction_t *direction)
{
    if (NEKOR_KVM_UNLIKELY(!direction))
        return -EINVAL;

    if (NEKOR_KVM_UNLIKELY(run->exit_reason != KVM_EXIT_MMIO))
        return -EPROTO;

    if (NEKOR_KVM_UNLIKELY(run->mmio.len > NEKOR_KVM_SIZEOF(run->mmio.data) ||
                           run->mmio.is_write > 1))
        return -EBADMSG;

    *direction = run->mmio.is_write ? NEKOR_KVM_MMIO_DIRECTION_WRITE :
                                      NEKOR_KVM_MMIO_DIRECTION_READ;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Validate a mapped run image without reading its active exit fields.
 *
 * This establishes the fixed image size and alignment requirements. Exit fields and variable
 * data ranges are validated when accessed.
 */
NEKOR_KVM_FORCEINLINE nekor_kvm_status_t
nekor_kvm_run_image_validate(const void *run_image, nekor_kvm_storage_size_t run_size)
{
    return nekor_kvm_run_validate(run_image, run_size);
}

/**
 * Classify the active exit and preserve its original KVM reason code.
 *
 * Port-I/O, MMIO, and halt exits receive distinct classifications. Other exits retain their
 * original reason code without reading the associated union arm.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_run_exit_classify(const void *run_image,
                                                             nekor_kvm_storage_size_t run_size,
                                                             nekor_kvm_run_exit_kind_t *exit_kind,
                                                             nekor_kvm_exit_reason_t *exit_reason)
{
    const struct kvm_run *run;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!exit_kind || !exit_reason))
        return -EINVAL;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;
    *exit_reason = run->exit_reason;

    switch (run->exit_reason) {
    case KVM_EXIT_HLT:
        *exit_kind = NEKOR_KVM_RUN_EXIT_KIND_HALT;
        break;
    case KVM_EXIT_IO:
        *exit_kind = NEKOR_KVM_RUN_EXIT_KIND_IO;
        break;
    case KVM_EXIT_MMIO:
        *exit_kind = NEKOR_KVM_RUN_EXIT_KIND_MMIO;
        break;
    default:
        *exit_kind = NEKOR_KVM_RUN_EXIT_KIND_OTHER;
        break;
    }

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Copy the port-I/O metadata from an active `KVM_EXIT_IO` arm.
 *
 * The direction, element width, offset arithmetic, and mapped data range are validated even
 * though this operation returns metadata only.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_run_io_exit_read(const void *run_image,
                                                            nekor_kvm_storage_size_t run_size,
                                                            nekor_kvm_io_exit_t *io_exit)
{
    const struct kvm_run *run;
    nekor_kvm_storage_size_t data_offset;
    nekor_kvm_storage_size_t data_size;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!io_exit))
        return -EINVAL;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;
    target_status = nekor_kvm_run_io_extent(run, run_size, &data_offset, &data_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    (void)data_offset;
    (void)data_size;

    *io_exit = (nekor_kvm_io_exit_t){0};
    io_exit->direction = run->io.direction == KVM_EXIT_IO_IN ? NEKOR_KVM_IO_DIRECTION_IN :
                                                               NEKOR_KVM_IO_DIRECTION_OUT;
    io_exit->element_size = (nekor_kvm_io_element_size_t)run->io.size;
    io_exit->port.value = run->io.port;
    io_exit->element_count = run->io.count;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Return the direction of an active MMIO operation.
 *
 * The active exit tag and the MMIO arm's boolean and data-length fields are checked before the
 * output enum is written.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_run_mmio_exit_classify(
    const void *run_image, nekor_kvm_storage_size_t run_size, nekor_kvm_mmio_direction_t *direction)
{
    const struct kvm_run *run;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!direction))
        return -EINVAL;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;

    return nekor_kvm_run_mmio_direction(run, direction);
}

/**
 * Copy the address and response length of an active MMIO read.
 *
 * This record intentionally contains no response payload. Userspace supplies that payload
 * separately through `nekor_kvm_run_mmio_read_response_write` before the next KVM run ioctl.
 */
NEKOR_KVM_HOT nekor_kvm_status_t
nekor_kvm_run_mmio_read_exit_read(const void *run_image, nekor_kvm_storage_size_t run_size,
                                  nekor_kvm_mmio_read_exit_t *mmio_read_exit)
{
    const struct kvm_run *run;
    nekor_kvm_mmio_direction_t direction;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!mmio_read_exit))
        return -EINVAL;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;
    target_status = nekor_kvm_run_mmio_direction(run, &direction);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    if (NEKOR_KVM_UNLIKELY(direction != NEKOR_KVM_MMIO_DIRECTION_READ))
        return -EPROTO;

    *mmio_read_exit = (nekor_kvm_mmio_read_exit_t){0};
    mmio_read_exit->address.value = run->mmio.phys_addr;
    mmio_read_exit->byte_count = (__u8)run->mmio.len;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Copy the address and guest-written bytes of an active MMIO write.
 *
 * The bridge record is zero-initialized before copying the meaningful bytes, so its unused tail
 * bytes have a defined value. The active direction and eight-byte UAPI data bound are checked
 * before the copy.
 */
NEKOR_KVM_HOT nekor_kvm_status_t
nekor_kvm_run_mmio_write_exit_read(const void *run_image, nekor_kvm_storage_size_t run_size,
                                   nekor_kvm_mmio_write_exit_t *mmio_write_exit)
{
    const struct kvm_run *run;
    nekor_kvm_mmio_direction_t direction;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!mmio_write_exit))
        return -EINVAL;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;
    target_status = nekor_kvm_run_mmio_direction(run, &direction);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    if (NEKOR_KVM_UNLIKELY(direction != NEKOR_KVM_MMIO_DIRECTION_WRITE))
        return -EPROTO;

    *mmio_write_exit = (nekor_kvm_mmio_write_exit_t){0};
    mmio_write_exit->address.value = run->mmio.phys_addr;
    mmio_write_exit->byte_count = (__u8)run->mmio.len;
    memcpy(mmio_write_exit->data, run->mmio.data, run->mmio.len);

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Write userspace response bytes into the active MMIO read arm.
 *
 * The exit must still be a guest read, and the response length must exactly match the byte count
 * requested by KVM. This updates only the response payload consumed by the next run ioctl.
 */
NEKOR_KVM_HOT nekor_kvm_status_t
nekor_kvm_run_mmio_read_response_write(void *run_image, nekor_kvm_storage_size_t run_size,
                                       const __u8 *data, nekor_kvm_storage_size_t data_size)
{
    struct kvm_run *run;
    nekor_kvm_mmio_direction_t direction;
    nekor_kvm_status_t target_status;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (struct kvm_run *)run_image;
    target_status = nekor_kvm_run_mmio_direction(run, &direction);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    if (NEKOR_KVM_UNLIKELY(direction != NEKOR_KVM_MMIO_DIRECTION_READ))
        return -EPROTO;

    if (NEKOR_KVM_UNLIKELY(data_size != run->mmio.len))
        return data_size < run->mmio.len ? -ENOBUFS : -EINVAL;

    if (NEKOR_KVM_UNLIKELY(run->mmio.len && !data))
        return -EINVAL;

    if (run->mmio.len)
        memcpy(run->mmio.data, data, run->mmio.len);

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Copy guest port-output bytes from the run image's flexible data area.
 *
 * The operation requires an active port-output exit and a destination large enough for every
 * element. The reported output length is initialized to zero and is set only after a successful
 * copy.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_run_io_output_read(
    const void *run_image, nekor_kvm_storage_size_t run_size, uint8_t *data,
    nekor_kvm_storage_size_t data_capacity, nekor_kvm_storage_size_t *data_size)
{
    const struct kvm_run *run;
    nekor_kvm_storage_size_t data_offset;
    nekor_kvm_storage_size_t required_size;
    nekor_kvm_status_t target_status;

    if (NEKOR_KVM_UNLIKELY(!data_size))
        return -EINVAL;

    *data_size = 0;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (const struct kvm_run *)run_image;
    target_status = nekor_kvm_run_io_extent(run, run_size, &data_offset, &required_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    if (NEKOR_KVM_UNLIKELY(run->io.direction != KVM_EXIT_IO_OUT))
        return -EPROTO;

    if (NEKOR_KVM_UNLIKELY(data_capacity < required_size))
        return -ENOBUFS;

    if (NEKOR_KVM_UNLIKELY(required_size && !data))
        return -EINVAL;

    if (required_size)
        memcpy(data, (const uint8_t *)run_image + data_offset, required_size);

    *data_size = required_size;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Copy userspace port-input bytes into the run image's flexible data area.
 *
 * The operation requires an active port-input exit and an exact byte count matching the validated
 * element width multiplied by element count. No bytes are written on a direction, size, or extent
 * error.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_run_io_input_write(void *run_image,
                                                              nekor_kvm_storage_size_t run_size,
                                                              const uint8_t *data,
                                                              nekor_kvm_storage_size_t data_size)
{
    struct kvm_run *run;
    nekor_kvm_storage_size_t data_offset;
    nekor_kvm_storage_size_t required_size;
    nekor_kvm_status_t target_status;

    target_status = nekor_kvm_run_validate(run_image, run_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    run = (struct kvm_run *)run_image;
    target_status = nekor_kvm_run_io_extent(run, run_size, &data_offset, &required_size);
    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return target_status;

    if (NEKOR_KVM_UNLIKELY(run->io.direction != KVM_EXIT_IO_IN))
        return -EPROTO;

    if (NEKOR_KVM_UNLIKELY(data_size != required_size))
        return data_size < required_size ? -ENOBUFS : -EINVAL;

    if (NEKOR_KVM_UNLIKELY(required_size && !data))
        return -EINVAL;

    if (required_size)
        memcpy((uint8_t *)run_image + data_offset, data, required_size);

    return NEKOR_KVM_STATUS_OK;
}
