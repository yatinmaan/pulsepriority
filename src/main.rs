use anyhow::Context;
use pulsepriority::{DeviceInfo, Manager};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "pulsepriority")]
struct Opt {
    /// Lists all output devices.
    #[structopt(short, long)]
    list: bool,

    /// Select index of device to be set as first priority.
    #[structopt(short = "s", long = "select")]
    index: Option<usize>,

    /// Disables priority based routing.
    #[structopt(short, long)]
    disable: bool,
}

fn display_devices(device_list: &[DeviceInfo]) {
    println! {"Index  Name"}
    for device in device_list {
        println! {"{:3}    {}", device.index, device.description}
    }
    println! {}
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let mut manager =
        Manager::new("pulsepriority").with_context(|| "Unable to connect to PulseAudio.")?;

    if opt.disable {
        manager.disable_priority_routing()?;
    }

    if opt.list {
        let priority_list = manager.get_priority_list()?;
        display_devices(&priority_list);
    }

    if let Some(index) = opt.index {
        let priority_list = manager.get_priority_list()?;
        anyhow::ensure!(
            index > 0 && index < priority_list.len(),
            format!("Invalid Index: {}", index)
        );
        // Convert to zero-based indexing
        let index = index - 1;

        let selected_device = vec![priority_list[index].clone()];
        let priority_list = manager.set_priority_list(selected_device)?;

        display_devices(&priority_list);
    }
    Ok(())
}
