use std::io::{self, Write};

use crate::sonar_runtime::SonarRuntime;
use crate::sonar_debug_console::SonarDebugConsole;
use crate::device::SonarDevice;

/// A simple CLI wrapper for the sonar runtime.
/// This is intentionally lightweight and standalone.
/// Later, you can replace this with a full TUI or GUI.
pub struct SonarCli<D: SonarDevice> {
    runtime: SonarRuntime<D>,
    console: SonarDebugConsole,
}

impl<D: SonarDevice> SonarCli<D> {
    pub fn new(runtime: SonarRuntime<D>) -> Self {
        Self {
            runtime,
            console: SonarDebugConsole::new(),
        }
    }

    /// Run the CLI loop.
    /// Commands:
    ///   step       - run one frame
    ///   run <n>    - run n frames
    ///   show       - print latest sonar output
    ///   hazard     - print latest sonar output (including hazard map)
    ///   quit       - exit CLI
    pub fn run(&mut self) {
        println!("Sonar CLI started.");
        println!("Commands: step | run <n> | show | hazard | quit");

        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                println!("Input error.");
                continue;
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Parse commands
            if input == "quit" {
                println!("Exiting sonar CLI.");
                break;
            }

            if input == "step" {
                self.runtime.tick();
                self.console.print_latest(&self.runtime);
                continue;
            }

            if input.starts_with("run ") {
                if let Some(n_str) = input.split_whitespace().nth(1) {
                    if let Ok(n) = n_str.parse::<u64>() {
                        self.runtime.run_frames(n);
                        self.console.print_latest(&self.runtime);
                        continue;
                    }
                }
                println!("Invalid run command. Usage: run <n>");
                continue;
            }

            if input == "show" {
                self.console.print_latest(&self.runtime);
                continue;
            }

            // Hazard: reuse console’s full debug output (includes hazard map)
            if input == "hazard" {
                self.console.print_latest(&self.runtime);
                continue;
            }

            println!("Unknown command: {}", input);
        }
    }
}
