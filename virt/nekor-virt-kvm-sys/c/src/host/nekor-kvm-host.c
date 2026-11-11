#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <linux/kvm.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

#include "nekor-kvm-miscenalleous.h"
#include "nekor-kvm.h"

NEKOR_KVM_STATIC_ASSERT(NEKOR_KVM_SIZEOF(nekor_kvm_descriptor_t) == NEKOR_KVM_SIZEOF(int),
                        "The Nekor descriptor type must match the host descriptor type.");
NEKOR_KVM_STATIC_ASSERT(NEKOR_KVM_SIZEOF(nekor_kvm_storage_size_t) == NEKOR_KVM_SIZEOF(size_t),
                        "The Nekor storage extent must match the compiler size type.");
NEKOR_KVM_STATIC_ASSERT(KVM_API_VERSION == 12, "The C host bridge requires KVM API version 12.");

/**
 * Convert the host thread's current error number to the bridge's signed status convention.
 *
 * The operation caller invokes this immediately after a failed host function so another call
 * cannot replace the thread-local error number first.
 */
static NEKOR_KVM_COLD nekor_kvm_status_t nekor_kvm_host_failure(void)
{
    const int target_error = errno ? errno : EIO;

    return (nekor_kvm_status_t)-target_error;
}

/**
 * Open the selected KVM device path and return its owned descriptor.
 *
 * The descriptor is opened for read and write access and marked close-on-exec. Ownership
 * transfers to the caller on success.
 */
nekor_kvm_status_t nekor_kvm_device_open(const char *device_path,
                                         nekor_kvm_descriptor_t *descriptor)
{
    int target_descriptor;

    if (NEKOR_KVM_UNLIKELY(!device_path || !descriptor))
        return -EINVAL;

    *descriptor = -1;
    target_descriptor = open(device_path, O_RDWR | O_CLOEXEC);

    if (NEKOR_KVM_UNLIKELY(target_descriptor < 0))
        return nekor_kvm_host_failure();

    *descriptor = (nekor_kvm_descriptor_t)target_descriptor;

    return 0;
}

/**
 * Read the API version reported by the KVM device.
 */
nekor_kvm_status_t nekor_kvm_device_api_version_read(nekor_kvm_descriptor_t descriptor,
                                                     nekor_kvm_api_version_t *api_version)
{
    int target_api_version;

    if (NEKOR_KVM_UNLIKELY(descriptor < 0 || !api_version))
        return -EINVAL;

    *api_version = 0;
    target_api_version = ioctl(descriptor, KVM_GET_API_VERSION);

    if (NEKOR_KVM_UNLIKELY(target_api_version < 0))
        return nekor_kvm_host_failure();

    *api_version = (nekor_kvm_api_version_t)target_api_version;

    return 0;
}

/**
 * Read the mapping extent required for one KVM vCPU run image.
 */
nekor_kvm_status_t nekor_kvm_device_run_mapping_size_read(nekor_kvm_descriptor_t descriptor,
                                                          nekor_kvm_storage_size_t *mapping_size)
{
    int target_mapping_size;

    if (NEKOR_KVM_UNLIKELY(descriptor < 0 || !mapping_size))
        return -EINVAL;

    *mapping_size = 0;
    target_mapping_size = ioctl(descriptor, KVM_GET_VCPU_MMAP_SIZE);

    if (NEKOR_KVM_UNLIKELY(target_mapping_size < 0))
        return nekor_kvm_host_failure();

    *mapping_size = (nekor_kvm_storage_size_t)target_mapping_size;

    return 0;
}

/**
 * Read the host page size used to align guest memory regions.
 */
nekor_kvm_status_t nekor_kvm_host_page_size_read(nekor_kvm_storage_size_t *page_size)
{
    long target_page_size;

    if (NEKOR_KVM_UNLIKELY(!page_size))
        return -EINVAL;

    *page_size = 0;
    target_page_size = sysconf(_SC_PAGESIZE);

    if (NEKOR_KVM_UNLIKELY(target_page_size <= 0))
        return nekor_kvm_host_failure();

    *page_size = (nekor_kvm_storage_size_t)target_page_size;

    return 0;
}

/**
 * Create one KVM virtual machine using the selected VM type.
 */
nekor_kvm_status_t nekor_kvm_vm_create(nekor_kvm_descriptor_t device_descriptor,
                                       nekor_kvm_vm_type_t vm_type,
                                       nekor_kvm_descriptor_t *vm_descriptor)
{
    int target_descriptor;

    if (NEKOR_KVM_UNLIKELY(device_descriptor < 0 || !vm_descriptor))
        return -EINVAL;

    *vm_descriptor = -1;
    target_descriptor = ioctl(device_descriptor, KVM_CREATE_VM, (unsigned long)vm_type);

    if (NEKOR_KVM_UNLIKELY(target_descriptor < 0))
        return nekor_kvm_host_failure();

    *vm_descriptor = (nekor_kvm_descriptor_t)target_descriptor;

    return 0;
}

/**
 * Create one KVM vCPU using its numeric identifier.
 */
nekor_kvm_status_t nekor_kvm_vcpu_create(nekor_kvm_descriptor_t vm_descriptor,
                                         nekor_kvm_vcpu_id_t vcpu_id,
                                         nekor_kvm_descriptor_t *vcpu_descriptor)
{
    int target_descriptor;

    if (NEKOR_KVM_UNLIKELY(vm_descriptor < 0 || !vcpu_descriptor))
        return -EINVAL;

    *vcpu_descriptor = -1;
    target_descriptor = ioctl(vm_descriptor, KVM_CREATE_VCPU, (unsigned long)vcpu_id);

    if (NEKOR_KVM_UNLIKELY(target_descriptor < 0))
        return nekor_kvm_host_failure();

    *vcpu_descriptor = (nekor_kvm_descriptor_t)target_descriptor;

    return 0;
}

/**
 * Register one host mapping as guest physical memory for the selected VM.
 */
nekor_kvm_status_t nekor_kvm_guest_memory_register(nekor_kvm_descriptor_t vm_descriptor,
                                                   nekor_kvm_memory_slot_t slot,
                                                   nekor_kvm_guest_physical_address_t guest_address,
                                                   nekor_kvm_guest_memory_size_t memory_size,
                                                   const void *host_mapping)
{
    struct kvm_userspace_memory_region kernel_region = {0};

    if (NEKOR_KVM_UNLIKELY(vm_descriptor < 0 || !host_mapping || !memory_size))
        return -EINVAL;

    kernel_region.slot = slot;
    kernel_region.flags = 0;
    kernel_region.guest_phys_addr = guest_address.value;
    kernel_region.memory_size = memory_size;
    kernel_region.userspace_addr = (__u64)(uintptr_t)host_mapping;

    if (NEKOR_KVM_UNLIKELY(ioctl(vm_descriptor, KVM_SET_USER_MEMORY_REGION, &kernel_region) < 0))
        return nekor_kvm_host_failure();

    return 0;
}

/**
 * Create a private, readable, and writable host mapping for guest RAM.
 */
nekor_kvm_status_t nekor_kvm_mapping_anonymous_create(nekor_kvm_storage_size_t mapping_size,
                                                      void **mapping)
{
    void *target_mapping;

    if (NEKOR_KVM_UNLIKELY(!mapping_size || !mapping))
        return -EINVAL;

    *mapping = NULL;
    target_mapping = mmap(NULL, (size_t)mapping_size, PROT_READ | PROT_WRITE,
                          MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);

    if (NEKOR_KVM_UNLIKELY(target_mapping == MAP_FAILED))
        return nekor_kvm_host_failure();

    if (NEKOR_KVM_UNLIKELY(!target_mapping)) {
        (void)munmap(target_mapping, (size_t)mapping_size);
        errno = EFAULT;
        return nekor_kvm_host_failure();
    }

    *mapping = target_mapping;

    return 0;
}

/**
 * Create a shared, readable, and writable mapping of a vCPU run image.
 */
nekor_kvm_status_t nekor_kvm_mapping_shared_create(nekor_kvm_descriptor_t vcpu_descriptor,
                                                   nekor_kvm_storage_size_t mapping_size,
                                                   void **mapping)
{
    void *target_mapping;

    if (NEKOR_KVM_UNLIKELY(vcpu_descriptor < 0 || !mapping_size || !mapping))
        return -EINVAL;

    *mapping = NULL;
    target_mapping =
        mmap(NULL, (size_t)mapping_size, PROT_READ | PROT_WRITE, MAP_SHARED, vcpu_descriptor, 0);

    if (NEKOR_KVM_UNLIKELY(target_mapping == MAP_FAILED))
        return nekor_kvm_host_failure();

    if (NEKOR_KVM_UNLIKELY(!target_mapping)) {
        (void)munmap(target_mapping, (size_t)mapping_size);
        errno = EFAULT;
        return nekor_kvm_host_failure();
    }

    *mapping = target_mapping;

    return 0;
}

/**
 * Release a host mapping returned by one of the KVM mapping operations.
 *
 * Destruction is best-effort because a mapping owner cannot report an unmap failure from its
 * destructor.
 */
void nekor_kvm_mapping_release(void *mapping, nekor_kvm_storage_size_t mapping_size)
{
    if (mapping && mapping_size)
        (void)munmap(mapping, (size_t)mapping_size);
}

/**
 * Configure KVM special and general registers for real-mode entry.
 *
 * Configuration stops at the first failed ioctl. Earlier register updates remain in effect.
 */
nekor_kvm_status_t nekor_kvm_vcpu_real_mode_entry_configure(
    nekor_kvm_descriptor_t vcpu_descriptor,
    nekor_kvm_real_mode_instruction_pointer_t instruction_pointer)
{
    struct kvm_sregs special_registers = {0};
    struct kvm_regs registers = {0};

    if (NEKOR_KVM_UNLIKELY(vcpu_descriptor < 0))
        return -EINVAL;

    if (NEKOR_KVM_UNLIKELY(ioctl(vcpu_descriptor, KVM_GET_SREGS, &special_registers) < 0))
        return nekor_kvm_host_failure();

    special_registers.cs.base = 0;
    special_registers.cs.limit = UINT16_MAX;
    special_registers.cs.selector = 0;
    special_registers.cs.type = 0b1011;
    special_registers.cs.present = 1;
    special_registers.cs.dpl = 0;
    special_registers.cs.db = 0;
    special_registers.cs.s = 1;
    special_registers.cs.l = 0;
    special_registers.cs.g = 0;
    special_registers.cs.avl = 0;
    special_registers.cs.unusable = 0;
    special_registers.cs.padding = 0;

    if (NEKOR_KVM_UNLIKELY(ioctl(vcpu_descriptor, KVM_SET_SREGS, &special_registers) < 0))
        return nekor_kvm_host_failure();

    if (NEKOR_KVM_UNLIKELY(ioctl(vcpu_descriptor, KVM_GET_REGS, &registers) < 0))
        return nekor_kvm_host_failure();

    registers.rip = instruction_pointer;
    registers.rflags |= 2;

    if (NEKOR_KVM_UNLIKELY(ioctl(vcpu_descriptor, KVM_SET_REGS, &registers) < 0))
        return nekor_kvm_host_failure();

    return 0;
}

/**
 * Enter KVM and run one vCPU until the kernel returns an exit or an error.
 *
 * The run mapping's size and alignment are validated before execution begins.
 */
NEKOR_KVM_HOT nekor_kvm_status_t nekor_kvm_vcpu_run(nekor_kvm_descriptor_t vcpu_descriptor,
                                                    void *run_mapping,
                                                    nekor_kvm_storage_size_t run_mapping_size)
{
    const nekor_kvm_status_t target_status =
        nekor_kvm_run_image_validate(run_mapping, run_mapping_size);

    if (NEKOR_KVM_UNLIKELY(vcpu_descriptor < 0))
        return -EINVAL;

    if (NEKOR_KVM_UNLIKELY(target_status != NEKOR_KVM_STATUS_OK))
        return -EINVAL;

    if (NEKOR_KVM_UNLIKELY(ioctl(vcpu_descriptor, KVM_RUN) < 0))
        return nekor_kvm_host_failure();

    return 0;
}
