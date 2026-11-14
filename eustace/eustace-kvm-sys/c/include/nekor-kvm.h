#ifndef NEKOR_KVM_H
#define NEKOR_KVM_H

#include <linux/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Signed operation result.
 *
 * Zero reports success. Failure returns the negated host error number.
 */
typedef __s32 nekor_kvm_status_t;

/**
 * Successful completion of an operation.
 */
static const nekor_kvm_status_t NEKOR_KVM_STATUS_OK = 0;

/**
 * Classification of an exit from the opaque KVM run image.
 */
typedef enum nekor_kvm_run_exit_kind {
    /**
     * No Nekor bridge record is available for this exit kind.
     */
    NEKOR_KVM_RUN_EXIT_KIND_OTHER = 0,
    /**
     * The active exit arm contains port-I/O state.
     */
    NEKOR_KVM_RUN_EXIT_KIND_IO = 1,
    /**
     * The active exit arm contains MMIO state.
     */
    NEKOR_KVM_RUN_EXIT_KIND_MMIO = 2,
    /**
     * The guest halted.
     */
    NEKOR_KVM_RUN_EXIT_KIND_HALT = 3
} nekor_kvm_run_exit_kind_t;

/**
 * Direction of a guest port-I/O operation.
 */
typedef enum nekor_kvm_io_direction {
    /**
     * The guest reads from the port and userspace supplies the value.
     */
    NEKOR_KVM_IO_DIRECTION_IN = 0,
    /**
     * The guest writes to the port and userspace consumes the value.
     */
    NEKOR_KVM_IO_DIRECTION_OUT = 1
} nekor_kvm_io_direction_t;

/**
 * Width of each element in a guest port-I/O operation.
 */
typedef enum nekor_kvm_io_element_size {
    /**
     * Each port-I/O element occupies one byte.
     */
    NEKOR_KVM_IO_ELEMENT_SIZE_BYTE = 1,
    /**
     * Each port-I/O element occupies two bytes.
     */
    NEKOR_KVM_IO_ELEMENT_SIZE_WORD = 2,
    /**
     * Each port-I/O element occupies four bytes.
     */
    NEKOR_KVM_IO_ELEMENT_SIZE_DWORD = 4
} nekor_kvm_io_element_size_t;

/**
 * Direction of a guest MMIO operation.
 */
typedef enum nekor_kvm_mmio_direction {
    /**
     * The guest reads from the addressed device.
     */
    NEKOR_KVM_MMIO_DIRECTION_READ = 0,
    /**
     * The guest writes to the addressed device.
     */
    NEKOR_KVM_MMIO_DIRECTION_WRITE = 1
} nekor_kvm_mmio_direction_t;

/**
 * Byte length of a storage region.
 */
typedef __kernel_size_t nekor_kvm_storage_size_t;

/**
 * A process file descriptor.
 */
typedef __s32 nekor_kvm_descriptor_t;

/**
 * Numeric identity of a virtual processor within a KVM virtual machine.
 */
typedef __u32 nekor_kvm_vcpu_id_t;

/**
 * Architecture-dependent VM type selected when creating a KVM virtual machine.
 */
typedef __u32 nekor_kvm_vm_type_t;

/**
 * KVM memory-slot identifier, including its address-space selector bits.
 */
typedef __u32 nekor_kvm_memory_slot_t;

/**
 * Instruction-pointer offset within the real-mode code segment.
 */
typedef __u16 nekor_kvm_real_mode_instruction_pointer_t;

/**
 * API version reported by the KVM device.
 */
typedef __s32 nekor_kvm_api_version_t;

/**
 * Byte size of a guest memory region.
 */
typedef __u64 nekor_kvm_guest_memory_size_t;

/**
 * Original KVM reason code, including unrecognized exit kinds.
 */
typedef __u32 nekor_kvm_exit_reason_t;

/**
 * An x86 port number, kept distinct from a guest memory address.
 */
typedef struct nekor_kvm_io_port {
    /**
     * The sixteen-bit port number.
     */
    __u16 value;
} nekor_kvm_io_port_t;

/**
 * A guest physical address, kept distinct from a host pointer.
 */
typedef struct nekor_kvm_guest_physical_address {
    /**
     * The address value in the guest physical address domain.
     */
    __u64 value;
} nekor_kvm_guest_physical_address_t;

/**
 * Selection fields for a CPUID leaf and subleaf.
 */
typedef struct nekor_kvm_cpuid_query {
    /**
     * CPUID function selected by the query.
     */
    __u32 leaf;

    /**
     * CPUID index selected by the query.
     */
    __u32 subleaf;

    /**
     * KVM query attributes associated with the selected leaf.
     */
    __u32 flags;
} nekor_kvm_cpuid_query_t;

/**
 * Four general-purpose registers returned for a CPUID query.
 */
typedef struct nekor_kvm_cpuid_registers {
    /**
     * Value returned in EAX.
     */
    __u32 eax;

    /**
     * Value returned in EBX.
     */
    __u32 ebx;

    /**
     * Value returned in ECX.
     */
    __u32 ecx;

    /**
     * Value returned in EDX.
     */
    __u32 edx;
} nekor_kvm_cpuid_registers_t;

/**
 * A CPUID query together with its returned register values.
 */
typedef struct nekor_kvm_cpuid_entry {
    /**
     * Leaf, subleaf, and KVM query attributes.
     */
    nekor_kvm_cpuid_query_t query;

    /**
     * Register values returned for the associated CPUID query.
     */
    nekor_kvm_cpuid_registers_t registers;
} nekor_kvm_cpuid_entry_t;

/**
 * Guest port-I/O operation fields needed by a userspace device handler.
 */
typedef struct nekor_kvm_io_exit {
    /**
     * Whether userspace supplies or consumes the port value.
     */
    nekor_kvm_io_direction_t direction;

    /**
     * Number of bytes in each repeated port-I/O element.
     */
    nekor_kvm_io_element_size_t element_size;

    /**
     * Port addressed by the guest operation.
     */
    nekor_kvm_io_port_t port;

    /**
     * Number of elements in the operation.
     */
    __u32 element_count;
} nekor_kvm_io_exit_t;

/**
 * Guest memory-mapped read request without response data.
 */
typedef struct nekor_kvm_mmio_read_exit {
    /**
     * Guest physical address read by the guest operation.
     */
    nekor_kvm_guest_physical_address_t address;

    /**
     * Number of response bytes requested, at most eight.
     */
    __u8 byte_count;
} nekor_kvm_mmio_read_exit_t;

/**
 * Guest memory-mapped write data supplied to a device handler.
 */
typedef struct nekor_kvm_mmio_write_exit {
    /**
     * Guest physical address written by the guest operation.
     */
    nekor_kvm_guest_physical_address_t address;

    /**
     * Data bytes written by the guest, with unused bytes zeroed.
     */
    __u8 data[8];

    /**
     * Number of meaningful data bytes, at most eight.
     */
    __u8 byte_count;
} nekor_kvm_mmio_write_exit_t;

/**
 * Open the selected KVM device path for read and write access.
 *
 * Success transfers ownership of the opened file descriptor to the caller.
 */
nekor_kvm_status_t nekor_kvm_device_open(const char *device_path,
                                         nekor_kvm_descriptor_t *descriptor);

/**
 * Read the API version reported by the opened KVM device.
 */
nekor_kvm_status_t nekor_kvm_device_api_version_read(nekor_kvm_descriptor_t descriptor,
                                                     nekor_kvm_api_version_t *api_version);

/**
 * Read the byte extent required to map one KVM vCPU run image.
 */
nekor_kvm_status_t nekor_kvm_device_run_mapping_size_read(nekor_kvm_descriptor_t descriptor,
                                                          nekor_kvm_storage_size_t *mapping_size);

/**
 * Read the host page size used to align guest memory mappings.
 */
nekor_kvm_status_t nekor_kvm_host_page_size_read(nekor_kvm_storage_size_t *page_size);

/**
 * Create a KVM virtual-machine descriptor from the opened KVM device.
 */
nekor_kvm_status_t nekor_kvm_vm_create(nekor_kvm_descriptor_t device_descriptor,
                                       nekor_kvm_vm_type_t vm_type,
                                       nekor_kvm_descriptor_t *vm_descriptor);

/**
 * Create a KVM vCPU descriptor for the requested vCPU identifier.
 */
nekor_kvm_status_t nekor_kvm_vcpu_create(nekor_kvm_descriptor_t vm_descriptor,
                                         nekor_kvm_vcpu_id_t vcpu_id,
                                         nekor_kvm_descriptor_t *vcpu_descriptor);

/**
 * Register a host mapping as one KVM guest physical memory region.
 */
nekor_kvm_status_t nekor_kvm_guest_memory_register(nekor_kvm_descriptor_t vm_descriptor,
                                                   nekor_kvm_memory_slot_t slot,
                                                   nekor_kvm_guest_physical_address_t guest_address,
                                                   nekor_kvm_guest_memory_size_t memory_size,
                                                   const void *host_mapping);

/**
 * Create a zero-filled private mapping for guest RAM.
 */
nekor_kvm_status_t nekor_kvm_mapping_anonymous_create(nekor_kvm_storage_size_t mapping_size,
                                                      void **mapping);

/**
 * Map the shared KVM run image associated with a vCPU descriptor.
 */
nekor_kvm_status_t nekor_kvm_mapping_shared_create(nekor_kvm_descriptor_t vcpu_descriptor,
                                                   nekor_kvm_storage_size_t mapping_size,
                                                   void **mapping);

/**
 * Copy bytes from a host mapping into caller-owned storage after checking the requested extent.
 */
nekor_kvm_status_t nekor_kvm_mapping_copy_to(const void *mapping,
                                             nekor_kvm_storage_size_t mapping_size,
                                             nekor_kvm_storage_size_t mapping_offset, __u8 *data,
                                             nekor_kvm_storage_size_t data_size);

/**
 * Copy caller-owned bytes into a host mapping after checking the requested extent.
 */
nekor_kvm_status_t nekor_kvm_mapping_copy_from(void *mapping, nekor_kvm_storage_size_t mapping_size,
                                               nekor_kvm_storage_size_t mapping_offset,
                                               const __u8 *data,
                                               nekor_kvm_storage_size_t data_size);

/**
 * Release a host mapping previously returned by a KVM bridge operation.
 */
void nekor_kvm_mapping_release(void *mapping, nekor_kvm_storage_size_t mapping_size);

/**
 * Configure the vCPU for the selected real-mode instruction pointer.
 */
nekor_kvm_status_t nekor_kvm_vcpu_real_mode_entry_configure(
    nekor_kvm_descriptor_t vcpu_descriptor,
    nekor_kvm_real_mode_instruction_pointer_t instruction_pointer);

/**
 * Resume the selected vCPU by issuing the KVM run operation.
 */
nekor_kvm_status_t nekor_kvm_vcpu_run(nekor_kvm_descriptor_t vcpu_descriptor, void *run_mapping,
                                      nekor_kvm_storage_size_t run_mapping_size);

/**
 * Query supported CPUID entries into a Nekor entry list.
 *
 * Success initializes the returned prefix of the entry list. Failure leaves the list unchanged.
 * Insufficient capacity returns `-E2BIG` so the caller can retry with a larger list.
 */
nekor_kvm_status_t nekor_kvm_cpuid_supported(nekor_kvm_descriptor_t device,
                                             nekor_kvm_cpuid_entry_t *entry_list,
                                             nekor_kvm_storage_size_t entry_capacity,
                                             nekor_kvm_storage_size_t *entry_count);

/**
 * Install a Nekor entry list as the selected vCPU's CPUID configuration.
 *
 * No pointer into the input list is retained after the call.
 */
nekor_kvm_status_t nekor_kvm_cpuid_install(nekor_kvm_descriptor_t vcpu,
                                           const nekor_kvm_cpuid_entry_t *entry_list,
                                           nekor_kvm_storage_size_t entry_count);

/**
 * Validate an opaque run image's size and alignment without inspecting its contents.
 */
nekor_kvm_status_t nekor_kvm_run_image_validate(const void *run_image,
                                                nekor_kvm_storage_size_t run_size);

/**
 * Classify the exit and return its original KVM reason code.
 */
nekor_kvm_status_t nekor_kvm_run_exit_classify(const void *run_image,
                                               nekor_kvm_storage_size_t run_size,
                                               nekor_kvm_run_exit_kind_t *exit_kind,
                                               nekor_kvm_exit_reason_t *exit_reason);

/**
 * Read the port-I/O arm after confirming that the active exit is port I/O.
 */
nekor_kvm_status_t nekor_kvm_run_io_exit_read(const void *run_image,
                                              nekor_kvm_storage_size_t run_size,
                                              nekor_kvm_io_exit_t *io_exit);

/**
 * Classify an active MMIO exit as a guest read or write.
 */
nekor_kvm_status_t nekor_kvm_run_mmio_exit_classify(const void *run_image,
                                                    nekor_kvm_storage_size_t run_size,
                                                    nekor_kvm_mmio_direction_t *direction);

/**
 * Read the address and response size of an active guest MMIO read.
 */
nekor_kvm_status_t nekor_kvm_run_mmio_read_exit_read(const void *run_image,
                                                     nekor_kvm_storage_size_t run_size,
                                                     nekor_kvm_mmio_read_exit_t *mmio_read_exit);

/**
 * Read the address and data of an active guest MMIO write.
 */
nekor_kvm_status_t nekor_kvm_run_mmio_write_exit_read(const void *run_image,
                                                      nekor_kvm_storage_size_t run_size,
                                                      nekor_kvm_mmio_write_exit_t *mmio_write_exit);

/**
 * Write response bytes for an active guest MMIO read.
 */
nekor_kvm_status_t nekor_kvm_run_mmio_read_response_write(void *run_image,
                                                          nekor_kvm_storage_size_t run_size,
                                                          const __u8 *data,
                                                          nekor_kvm_storage_size_t data_size);

/**
 * Copy guest port-output bytes from the flexible data region of the run image.
 */
nekor_kvm_status_t nekor_kvm_run_io_output_read(const void *run_image,
                                                nekor_kvm_storage_size_t run_size, __u8 *data,
                                                nekor_kvm_storage_size_t data_capacity,
                                                nekor_kvm_storage_size_t *data_size);

/**
 * Copy userspace port-input bytes into the flexible data region of the run image.
 */
nekor_kvm_status_t nekor_kvm_run_io_input_write(void *run_image, nekor_kvm_storage_size_t run_size,
                                                const __u8 *data,
                                                nekor_kvm_storage_size_t data_size);

#ifdef __cplusplus
}
#endif

#endif /* NEKOR_KVM_H */
