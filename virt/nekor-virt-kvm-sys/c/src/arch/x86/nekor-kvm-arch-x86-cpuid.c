#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/ioctl.h>

#include <linux/kvm.h>

#include "nekor-kvm-miscenalleous.h"
#include "nekor-kvm.h"

/**
 * Allocate an in-band CPUID2 header and flexible array for this operation.
 *
 * The capacity must fit both KVM's entry counter and the host allocation extent. Zeroed
 * allocation initializes the reserved header and entry fields before any ioctl can read them.
 */
static nekor_kvm_status_t nekor_kvm_cpuid_allocate(size_t entry_capacity,
                                                   struct kvm_cpuid2 **native)
{
    const size_t header_size = NEKOR_KVM_SIZEOF(struct kvm_cpuid2);
    const size_t entry_size = NEKOR_KVM_SIZEOF(struct kvm_cpuid_entry2);
    size_t allocation_size;

    if (NEKOR_KVM_UNLIKELY(entry_capacity > UINT32_MAX ||
                           entry_capacity > (SIZE_MAX - header_size) / entry_size))
        return -EOVERFLOW;

    allocation_size = header_size + entry_capacity * entry_size;
    *native = calloc(1, allocation_size);

    if (NEKOR_KVM_UNLIKELY(!*native))
        return -ENOMEM;

    (*native)->nent = (__u32)entry_capacity;

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Query supported CPUID values and translate the successful native result in bulk.
 *
 * The entry list is not touched until the ioctl succeeds and its returned count fits the
 * supplied capacity. Every return path releases the temporary native allocation. The host error
 * number is captured before release so cleanup cannot overwrite the failed ioctl's error.
 */
nekor_kvm_status_t nekor_kvm_cpuid_supported(nekor_kvm_descriptor_t device,
                                             nekor_kvm_cpuid_entry_t *entry_list,
                                             nekor_kvm_storage_size_t entry_capacity,
                                             nekor_kvm_storage_size_t *entry_count)
{
    struct kvm_cpuid2 *native;
    nekor_kvm_status_t status;

    if (NEKOR_KVM_UNLIKELY(device < 0 || !entry_count || (!entry_list && entry_capacity)))
        return -EINVAL;

    status = nekor_kvm_cpuid_allocate(entry_capacity, &native);

    if (NEKOR_KVM_UNLIKELY(status != NEKOR_KVM_STATUS_OK))
        return status;

    if (NEKOR_KVM_UNLIKELY(ioctl(device, KVM_GET_SUPPORTED_CPUID, native) < 0)) {
        status = -errno;
        free(native);
        return status;
    }

    if (NEKOR_KVM_UNLIKELY(native->nent > entry_capacity)) {
        free(native);
        return -EOVERFLOW;
    }

    for (size_t index = 0; index < native->nent; ++index) {
        const struct kvm_cpuid_entry2 *source = &native->entries[index];

        entry_list[index] = (nekor_kvm_cpuid_entry_t){
            .query =
                {
                    .leaf = source->function,
                    .subleaf = source->index,
                    .flags = source->flags,
                },
            .registers =
                {
                    .eax = source->eax,
                    .ebx = source->ebx,
                    .ecx = source->ecx,
                    .edx = source->edx,
                },
        };
    }

    *entry_count = native->nent;
    free(native);

    return NEKOR_KVM_STATUS_OK;
}

/**
 * Install a borrowed Nekor entry list through a temporary native CPUID2 image.
 *
 * Reserved kernel fields are zeroed before the ioctl. The temporary allocation is released on
 * success and failure, and no pointer into the caller's entry list is retained.
 */
nekor_kvm_status_t nekor_kvm_cpuid_install(nekor_kvm_descriptor_t vcpu,
                                           const nekor_kvm_cpuid_entry_t *entry_list,
                                           nekor_kvm_storage_size_t entry_count)
{
    struct kvm_cpuid2 *native;
    nekor_kvm_status_t status;

    if (NEKOR_KVM_UNLIKELY(vcpu < 0 || (!entry_list && entry_count)))
        return -EINVAL;

    status = nekor_kvm_cpuid_allocate(entry_count, &native);

    if (NEKOR_KVM_UNLIKELY(status != NEKOR_KVM_STATUS_OK))
        return status;

    for (size_t index = 0; index < entry_count; ++index) {
        const nekor_kvm_cpuid_entry_t *source = &entry_list[index];
        struct kvm_cpuid_entry2 *destination = &native->entries[index];

        destination->function = source->query.leaf;
        destination->index = source->query.subleaf;
        destination->flags = source->query.flags;
        destination->eax = source->registers.eax;
        destination->ebx = source->registers.ebx;
        destination->ecx = source->registers.ecx;
        destination->edx = source->registers.edx;
    }

    status = ioctl(vcpu, KVM_SET_CPUID2, native) < 0 ? -errno : NEKOR_KVM_STATUS_OK;
    free(native);

    return status;
}
