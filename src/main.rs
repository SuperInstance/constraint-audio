use constraint_audio::synth::ConstraintSynth;

fn main() {
    let preset = constraint_audio::builtin_presets()[0].clone();
    let mut synth = ConstraintSynth::with_preset(44100.0, preset);
    let buffer = synth.play_note(60, 100, 1.0);
    println!("Generated {} samples", buffer.len());
    println!(
        "Peak: {}",
        buffer.iter().fold(0.0f64, |a, &b| a.max(b.abs()))
    );
}
