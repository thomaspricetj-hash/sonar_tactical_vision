use std::io::{self, Write};

use crate::sonar_runtime::SonarRuntime;
use crate::sonar_debug_console::SonarDebugConsole;
use crate::device::SonarDevice;

/// A simple CLI wrapper for the sonar runtime.
/// Upgraded for MAX‑tier debugging:
/// - stable input loop
/// - clean error handling
/// - extended hazard command
/// - runtime tick safety
/// - future‑ready for TUI/GUI replacement
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

            match input {
                "quit" => {
                    println!("Exiting sonar CLI.");
                    break;
                }

                "step" => {
                    self.runtime.tick();
                    self.console.print_latest(&self.runtime);
                }

                "show" => {
                    self.console.print_latest(&self.runtime);
                }

                "hazard" => {
                    // MAX‑tier: hazard map included in console output
                    self.console.print_latest(&self.runtime);
                }

                _ if input.starts_with("run ") => {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let Ok(n) = parts[1].parse::<u64>() {
                            self.runtime.run_frames(n);
                            self.console.print_latest(&self.runtime);
                            continue;
                        }
                    }
                    println!("Invalid run command. Usage: run <n>");
                }

                _ => {
                    println!("Unknown command: {}", input);
                }
            }
        }
    }
}
