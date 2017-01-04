use std::process::Command;

pub fn spawn_env() {
    let s = Command::new("docker")
            .arg("run")
            .arg("-p").arg("5900:5900")
            .arg("-p").arg("15900:15900")
            .arg("quay.io/openai/universe.gym-core:0.20.0")
            .spawn();

    print!("{}", "spawned docker process");
}
