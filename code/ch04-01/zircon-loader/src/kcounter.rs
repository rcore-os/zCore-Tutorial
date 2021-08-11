use {
    alloc::sync::Arc,
    core::mem::size_of,
    zircon_object::{object::KernelObject, vm::*},
};

pub fn create_kcounter_vmo() -> (Arc<VmObject>, Arc<VmObject>) {
    const HEADER_SIZE: usize = size_of::<KCounterVmoHeader>();
    let counter_name_vmo = VmObject::new_paged(1);
    let header = KCounterVmoHeader {
        magic: KCOUNTER_MAGIC,
        max_cpu: 1,
        counter_table_size: 0,
    };
    let serde_header: [u8; HEADER_SIZE] = unsafe { core::mem::transmute(header) };
    counter_name_vmo.write(0, &serde_header).unwrap();
    counter_name_vmo.set_name("counters/desc");

    let kcounters_vmo = VmObject::new_paged(1);
    kcounters_vmo.set_name("counters/arena");
    (counter_name_vmo, kcounters_vmo)
}

// #[repr(C)]
// struct KCounterDescItem {
//     name: [u8; 56],
//     type_: KCounterType,
// }

// #[repr(u64)]
// enum KCounterType {
//     Sum = 1,
// }

// impl From<&KCounterDescriptor> for KCounterDescItem {
//     fn from(desc: &KCounterDescriptor) -> Self {
//         let mut name = [0u8; 56];
//         let length = desc.name.len().min(56);
//         name[..length].copy_from_slice(&desc.name.as_bytes()[..length]);
//         KCounterDescItem {
//             name,
//             type_: KCounterType::Sum,
//         }
//     }
// }

#[repr(C)]
struct KCounterVmoHeader {
    magic: u64,
    max_cpu: u64,
    counter_table_size: usize,
}

const KCOUNTER_MAGIC: u64 = 1_547_273_975;
