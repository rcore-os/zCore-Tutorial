use {
    super::*,
    zircon_object::ipc::{Channel, MessagePacket},
};

impl Syscall<'_> {
    #[allow(clippy::too_many_arguments)]
    /// Read/Receive a message from a channel.  
    pub fn sys_channel_read(
        &self,
        handle_value: HandleValue,
        options: u32,
        mut bytes: UserOutPtr<u8>,
        handles: usize,
        num_bytes: u32,
        num_handles: u32,
        mut actual_bytes: UserOutPtr<u32>,
        mut actual_handles: UserOutPtr<u32>,
    ) -> ZxResult {
        info!(
            "channel.read: handle={:#x?}, options={:?}, bytes=({:#x?}; {:#x?}), handles=({:#x?}; {:#x?})",
            handle_value, options, bytes, num_bytes, handles, num_handles,
        );
        let proc = self.thread.proc();
        let channel = proc.get_object_with_rights::<Channel>(handle_value, Rights::READ)?;
        // FIX ME:
        const MAY_DISCARD: u32 = 1;
        let never_discard = options & MAY_DISCARD == 0;

        let msg = if never_discard {
            channel.check_and_read(|front_msg| {
                if num_bytes < front_msg.data.len() as u32
                    || num_handles < front_msg.handles.len() as u32
                {
                    let bytes = front_msg.data.len();
                    actual_bytes.write_if_not_null(bytes as u32)?;
                    actual_handles.write_if_not_null(front_msg.handles.len() as u32)?;
                    Err(ZxError::BUFFER_TOO_SMALL)
                } else {
                    Ok(())
                }
            })?
        } else {
            channel.read()?
        };

        // 如果要过 core-tests 把这个打开
        // hack_core_tests(handle_value, &self.thread.proc().name(), &mut msg.data);

        actual_bytes.write_if_not_null(msg.data.len() as u32)?;
        actual_handles.write_if_not_null(msg.handles.len() as u32)?;
        if num_bytes < msg.data.len() as u32 || num_handles < msg.handles.len() as u32 {
            return Err(ZxError::BUFFER_TOO_SMALL);
        }
        bytes.write_array(msg.data.as_slice())?;
        let values = proc.add_handles(msg.handles);
        UserOutPtr::<HandleValue>::from(handles).write_array(&values)?;
        Ok(())
    }

    /// Write a message to a channel.  
    pub fn sys_channel_write(
        &self,
        handle_value: HandleValue,
        options: u32,
        user_bytes: UserInPtr<u8>,
        num_bytes: u32,
        user_handles: UserInPtr<HandleValue>,
        num_handles: u32,
    ) -> ZxResult {
        info!(
            "channel.write: handle_value={:#x}, num_bytes={:#x}, num_handles={:#x}",
            handle_value, num_bytes, num_handles,
        );
        if options != 0 {
            return Err(ZxError::INVALID_ARGS);
        }
        if num_bytes > 65536 {
            return Err(ZxError::OUT_OF_RANGE);
        }
        let proc = self.thread.proc();
        let data = user_bytes.read_array(num_bytes as usize)?;
        let handles = user_handles.read_array(num_handles as usize)?;
        let transfer_self = handles.iter().any(|&handle| handle == handle_value);
        let handles = proc.remove_handles(&handles)?;
        if transfer_self {
            return Err(ZxError::NOT_SUPPORTED);
        }
        if handles.len() > 64 {
            return Err(ZxError::OUT_OF_RANGE);
        }
        for handle in handles.iter() {
            if !handle.rights.contains(Rights::TRANSFER) {
                return Err(ZxError::ACCESS_DENIED);
            }
        }
        let channel = proc.get_object_with_rights::<Channel>(handle_value, Rights::WRITE)?;
        channel.write(MessagePacket { data, handles })?;
        Ok(())
    }

    /// Create a new channel.   
    pub fn sys_channel_create(
        &self,
        options: u32,
        mut out0: UserOutPtr<HandleValue>,
        mut out1: UserOutPtr<HandleValue>,
    ) -> ZxResult {
        info!("channel.create: options={:#x}", options);
        if options != 0u32 {
            return Err(ZxError::INVALID_ARGS);
        }
        let proc = self.thread.proc();
        let (end0, end1) = Channel::create();
        let handle0 = proc.add_handle(Handle::new(end0, Rights::DEFAULT_CHANNEL));
        let handle1 = proc.add_handle(Handle::new(end1, Rights::DEFAULT_CHANNEL));
        out0.write(handle0)?;
        out1.write(handle1)?;
        Ok(())
    }
}

// HACK: pass arguments to standalone-test
// #[allow(clippy::naive_bytecount)]
// fn hack_core_tests(handle: HandleValue, thread_name: &str, data: &mut Vec<u8>) {
//     if handle == 3 && thread_name == "userboot" {
//         let cmdline = core::str::from_utf8(data).unwrap();
//         for kv in cmdline.split('\0') {
//             if let Some(v) = kv.strip_prefix("core-tests=") {
//                 *TESTS_ARGS.lock() = format!("test\0-f\0{}\0", v.replace(',', ":"));
//             }
//         }
//     } else if handle == 3 && thread_name == "test/core-standalone-test" {
//         let test_args = &*TESTS_ARGS.lock();
//         let len = data.len();
//         data.extend(test_args.bytes());
//         #[repr(C)]
//         #[derive(Debug)]
//         struct ProcArgs {
//             protocol: u32,
//             version: u32,
//             handle_info_off: u32,
//             args_off: u32,
//             args_num: u32,
//             environ_off: u32,
//             environ_num: u32,
//         }
//         #[allow(unsafe_code)]
//         #[allow(clippy::cast_ptr_alignment)]
//         let header = unsafe { &mut *(data.as_mut_ptr() as *mut ProcArgs) };
//         header.args_off = len as u32;
//         header.args_num = test_args.as_bytes().iter().filter(|&&b| b == 0).count() as u32;
//         warn!("HACKED: test args = {:?}", test_args);
//     }
// }
