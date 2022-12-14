use std::{sync::Mutex, time::Instant, vec, process::Child};
use crate::result::{Result, bail};
use std::io::{BufReader};
use std::sync::Arc;
use std::io::BufRead;
use std::sync::mpsc::{channel};
use std::time::Duration;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
static PROCESS_DELAY: u64 = 200;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectDescriptor {
  pub name: String,
  pub executable: String,
  pub workdir: String,
}

// todo: to be refactored
pub struct Project {
  pub name: String,
  pub executable: String,
  pub workdir: String,
  pub output: Arc<Mutex<Vec<String>>>,
  pub child: Option<Child>,
  pub offset: Arc<Mutex<i32>>,
  pub status: Arc<Mutex<ProcessStatus>>,
}

impl From<ProjectDescriptor> for Project {
  fn from(descriptor: ProjectDescriptor) -> Self {
    Project::new(descriptor.name, descriptor.executable, descriptor.workdir)
  }
}

pub struct ProcessStatus {
  pub started_at: Option<Instant>,
  pub is_running: bool,
}

impl Project {
  pub fn new(name: String, executable: String, workdir: String) -> Self {
    Project {
      name,
      executable,
      workdir,
      output: Arc::new(Mutex::new(vec![])),
      child: None,
      offset: Arc::new(Mutex::new(0)),
      status: Arc::new(
        Mutex::new(
          ProcessStatus {
            is_running: false,
            started_at: None
          }
        )
      )
    }
  }

  pub fn stop(&mut self) -> Result<bool>  {
    let mut status = self.status.lock().unwrap();

    if !status.is_running {
      return Ok(false);
    }

    if let Some(mut child) = self.child.take() {
      status.is_running = false;
      self.output.lock().unwrap().clear();
      child.kill()?;
      child.wait()?;

      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let mut status = self.status.lock().unwrap();

    if status.is_running {
      return Ok(());
    }

    status.is_running = true;
    status.started_at = Some(Instant::now());
    let mut child = Command::new("/bin/bash")
      .arg("-c")
      .arg(self.executable.as_str())
      .current_dir(self.workdir.as_str())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .stdin(Stdio::null())
      .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (sender, receiver) = channel();
    let stdout_sender = sender.clone();

    // todo: to be refactored
    let project_status = self.status.clone();
    let project_output = self.output.clone();
    // TODO: push data into temp vector and update it each n seconds
    std::thread::spawn(move || {
      let reader = BufReader::new(stdout);

      for line in reader.lines() {
        stdout_sender.send(line.unwrap()).unwrap();
      }

      let mut status = project_status.lock().unwrap();
      status.started_at = None;
      status.is_running = false;
      project_output.lock().unwrap().clear();
    });

    let stderr_sender = sender.clone();
    std::thread::spawn(move || {
      let reader = BufReader::new(stderr);

      for line in reader.lines() {
        stderr_sender.send(line.unwrap()).unwrap();
      }
    });

    let out = self.output.clone();
    let offset = self.offset.clone();
    std::thread::spawn(move || {
      loop {
        let mut buff = vec![];

        for line in receiver.try_iter() {
          buff.push(line);
        }

        // todo: store output in lines instead of string
        if !buff.is_empty() {
          let mut data = out.lock().unwrap();
          data.append(&mut buff);

          if data.len() > 10_000 {
            let (_, remain) = data.split_at(data.len() - 5_000);
            *data = remain.to_vec();
          }
        }

        std::thread::sleep(Duration::from_millis(PROCESS_DELAY));
      }
    });

    self.child = Some(child);

    Ok(())
  }

  pub fn lines(&mut self, width: u16) -> Vec<String> {
    self.output.lock()
      .unwrap()
      .iter()
      .flat_map(|line| {
        let mut chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
          chars.push('\n');
        }
        chars.chunks(width as usize)
          .map(|ch| {
            ch.into_iter().collect::<String>()
          })
          .collect::<Vec<String>>()
      })
      // .map(str::to_owned)
      .collect()
  }
}
