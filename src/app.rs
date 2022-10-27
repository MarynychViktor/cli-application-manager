use std::{sync::Mutex, time::Instant, vec, process::Child, io};
use std::result::{Result};
use std::error::{Error};


pub struct App {
  pub content: String,
  pub projects: Vec<Project>,
  pub active_project: Option<u8>
}

impl Default for App {
  fn default() -> Self {
    App {
      content: String::new(),
      projects: vec![],
      active_project: None
    }
  }
}

impl App {
  pub fn lines(&mut self, width: u16) -> Vec<String> {
    self.content.lines()
      .flat_map(|line| {
        let mut curr = line;
        let mut sublines = vec![];

        while curr.len() > width as usize {
            let (a,b) = curr.split_at(width as usize);
            sublines.push(a);
            curr = b;
        }

        if !curr.is_empty() {
            sublines.push(curr);
        }

        sublines
      })
      .map(str::to_owned)
      .collect()
  }

  pub fn select_next(&mut self) {
    if self.projects.is_empty() {
      return;
    }

    if let Some(index) = self.active_project {
        self.active_project = Some((index + 1).min((self.projects.len() - 1) as u8));
    } else {
      self.active_project = Some(0);
    }
  }

  pub fn select_prev(&mut self) {
    if self.projects.is_empty() {
      return;
    }

    if let Some(index) = self.active_project {
        self.active_project = Some(((index as i8) - 1).max(0) as u8);
    } else {
      self.active_project = Some(0);
    }
  }
}

pub struct Project {
  pub name: String,
  pub executable: String,
  pub workdir: String,
  pub output: Mutex<String>,
  pub started_at: Option<Instant>,
  pub finished_at: Option<Instant>,
  child: Option<Child>,
}

impl Project {
  pub fn new(name: String, executable: String, workdir: String) -> Self {
    Project {
      name,
      executable,
      workdir,
      output: Mutex::default(),
      started_at: None,
      finished_at: None,
      child: None
    }
  }

  pub fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
    if self.started_at.is_some() {
      return Err(Box::new(std::fmt::Error));
    }
  
    use std::process::{Command, Stdio};

    self.started_at = Some(Instant::now());
    let child = Command::new("/bin/bash")
      .arg("-c")
      .arg(self.executable.as_str())
      .current_dir(self.workdir.as_str())
      .stdout(Stdio::piped())
      .spawn()?;

    self.child = Some(child);

    Ok(())
  }

  pub fn lines(&mut self, width: u16) -> Vec<String> {
    self.output.lines()
      .flat_map(|line| {
        let mut curr = line;
        let mut sublines = vec![];

        while curr.len() > width as usize {
            let (a,b) = curr.split_at(width as usize);
            sublines.push(a);
            curr = b;
        }

        if !curr.is_empty() {
            sublines.push(curr);
        }

        sublines
      })
      .map(str::to_owned)
      .collect()
  }
}