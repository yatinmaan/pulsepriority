use anyhow::Context;
use closure::closure;

use libpulse_binding as pulse;
use pulse::context::ext_device_manager::DeviceManager;
use pulse::context::Context as Ctx;
use pulse::mainloop::standard::{IterateResult, Mainloop};
use pulse::operation::Operation;
use pulse::proplist::Proplist;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

pub struct Manager {
    device_manager: DeviceManager,
    mainloop: Rc<RefCell<Mainloop>>,
}

impl Manager {
    pub fn new(app_name: &str) -> anyhow::Result<Manager> {
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, app_name)
            .unwrap();

        let mainloop = Rc::new(RefCell::new(
            Mainloop::new().with_context(|| "Failed to create mainloop")?,
        ));

        let context = Rc::new(RefCell::new(
            Ctx::new_with_proplist(mainloop.borrow().deref(), "Context", &proplist)
                .with_context(|| "Failed to create mainloop")?,
        ));

        context
            .borrow_mut()
            .connect(None, pulse::context::flags::NOFLAGS, None)
            .with_context(|| "Failed to connect context")?;

        // Wait for context to be ready
        loop {
            match mainloop.borrow_mut().iterate(false) {
                IterateResult::Quit(_) => {
                    // TODO: Maybe use a better/custom error?
                    let unknown_err = pulse::error::Code::Unknown;
                    Err(pulse::error::PAErr::from(unknown_err))
                }
                IterateResult::Err(err) => Err(err),
                IterateResult::Success(_) => Ok(()),
            }?;
            match context.borrow().get_state() {
                pulse::context::State::Ready => {
                    break;
                }
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    let unknown_err = pulse::error::Code::Unknown;
                    Err(pulse::error::PAErr::from(unknown_err))
                }
                _ => Ok(()),
            }
            .with_context(|| "Context state failed/terminated.")?;
        }

        let mut introspector = context.borrow_mut().introspect();

        // TODO: Only load if its not already loaded?
        introspector.load_module("module-device-manager", "", |_| { /*TODO handle errors*/ });

        let mut device_manager = context.borrow_mut().device_manager();
        device_manager.enable_role_device_priority_routing(true, |_| { /*TODO handle errors*/ });

        /*
        TODO Refactor
        .load_module() & .enable_role_device_priority_routing() here return
        Operations that are actually executed whenever .run_loop() is called.
        This works for now, but isn't always guranteed to work.
        */

        Ok(Manager {
            mainloop,
            device_manager,
        })
    }

    pub fn get_priority_list(&mut self) -> anyhow::Result<Vec<DeviceInfo>> {
        // TODO: Check if RefCell is really required here.
        let devices: Rc<RefCell<Vec<DeviceInfo>>> = Rc::new(RefCell::new(Vec::new()));
        let read = &self.device_manager.read(closure!(
            clone devices, |info| {
                match info {
                    pulse::callbacks::ListResult::Item(res) => {
                        let device = DeviceInfo::new(
                            res.name.as_ref().unwrap().to_string(),
                            res.description.as_ref().unwrap().to_string(),
                            res.role_priorities[0].priority
                        );
                        // Ignore "source" devices.
                        if &device.name[0..4] == "sink"{
                            devices.borrow_mut().push(device);
                        }
                    },
                    pulse::callbacks::ListResult::End |
                    pulse::callbacks::ListResult::Error => {/*TODO handle errors*/}
                }
            }
        ));
        self.run_loop(vec![read])?;

        let mut devices = devices.borrow_mut().to_vec();
        // Sort the priority list by device index
        devices.sort_by_key(|device| device.index);
        Ok(devices)
    }

    pub fn set_priority_list(
        &mut self,
        device_order: Vec<DeviceInfo>,
    ) -> Result<Vec<DeviceInfo>, anyhow::Error> {
        let mut dev_names: Vec<&str> = Vec::new();
        for dev in &device_order {
            dev_names.push(&dev.name.as_str());
        }

        let update_priorities = &self.device_manager.reorder_devices_for_role(
            "none",
            &dev_names,
            |_| { /*TODO handle errors*/ },
        );
        self.run_loop(vec![&update_priorities])?;

        // Return the now updated priority list
        self.get_priority_list()
    }

    pub fn disable_priority_routing(&mut self) -> anyhow::Result<()> {
        let disable = self
            .device_manager
            .enable_role_device_priority_routing(false, |_| { /*TODO handle errors*/ });
        self.run_loop(vec![&disable])?;
        Ok(())
    }

    // TODO: Maybe turn this into a exec!{} / await!{} macro?
    fn run_loop<T, R>(&self, operations: Vec<&Operation<T>>) -> anyhow::Result<()>
    where
        T: FnMut(R) + ?Sized,
    {
        let mut pending_ops = operations.len();
        while pending_ops > 0 {
            match self.mainloop.borrow_mut().iterate(false) {
                IterateResult::Quit(_) => {
                    let unknown_err = pulse::error::Code::Unknown;
                    Err(pulse::error::PAErr::from(unknown_err))
                }
                IterateResult::Err(err) => Err(err),
                IterateResult::Success(_) => Ok(()),
            }?;

            for operation in &operations {
                match operation.get_state() {
                    pulse::operation::State::Running => {}
                    pulse::operation::State::Cancelled => {
                        pending_ops -= 1;
                    }
                    pulse::operation::State::Done => {
                        pending_ops -= 1;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub description: String,
    pub index: u32,
}

impl DeviceInfo {
    fn new(name: String, description: String, index: u32) -> Self {
        Self {
            name,
            description,
            index,
        }
    }
}
