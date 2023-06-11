use one_pole::OnePole;
use nih_plug::prelude::nih_export_standalone;

fn main() {
    nih_export_standalone::<OnePole<2>>();
}