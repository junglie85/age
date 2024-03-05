use age::{AgeResult, App, Game};

struct Sandbox {}

impl Sandbox {
    fn new(_app: &App) -> AgeResult<Self> {
        Ok(Self {})
    }
}

impl Game for Sandbox {}

fn main() {
    let Ok(app) = App::new(1920, 1080) else {
        return;
    };

    let Ok(sandbox) = Sandbox::new(&app) else {
        return;
    };

    if let Err(err) = app.run(sandbox) {
        eprintln!("Sandbox exited with error: {err}");
    }
}
