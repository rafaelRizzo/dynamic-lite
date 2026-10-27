//! Máquina de estados do Pomodoro. Pura: o relógio entra por parâmetro (epoch ms), o scheduler cuida do resto.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Focus,
    Short,
    Long,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Running,
    Paused,
}

/// Durações em ms e a cada quantos focos vem a pausa longa.
#[derive(Clone, Copy)]
pub struct Durations {
    pub focus: u64,
    pub short: u64,
    pub long: u64,
    pub long_every: u32,
}

impl Durations {
    pub fn of(&self, phase: Phase) -> u64 {
        match phase {
            Phase::Focus => self.focus,
            Phase::Short => self.short,
            Phase::Long => self.long,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Pomodoro {
    pub status: Status,
    pub phase: Phase,
    /// epoch ms em que a fase acaba (só com `Running`)
    pub ends_at: u64,
    /// ms que faltavam quando pausou (só com `Paused`)
    pub remaining_ms: u64,
    /// focos concluídos desde a última pausa longa
    pub cycle: u32,
}

impl Default for Pomodoro {
    fn default() -> Self {
        Self { status: Status::Idle, phase: Phase::Focus, ends_at: 0, remaining_ms: 0, cycle: 0 }
    }
}

impl Pomodoro {
    /// Idle: começa a fase atual do zero. Pausado: continua de onde parou.
    pub fn start(&mut self, now: u64, d: &Durations) {
        match self.status {
            Status::Running => {}
            Status::Paused => {
                self.ends_at = now + self.remaining_ms;
                self.status = Status::Running;
            }
            Status::Idle => {
                self.ends_at = now + d.of(self.phase);
                self.status = Status::Running;
            }
        }
    }

    pub fn pause(&mut self, now: u64) {
        if self.status == Status::Running {
            self.remaining_ms = self.ends_at.saturating_sub(now);
            self.status = Status::Paused;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_due(&self, now: u64) -> bool {
        self.status == Status::Running && now >= self.ends_at
    }

    /// Encerra a fase atual e escolhe a próxima. Devolve a fase que acabou.
    pub fn advance(&mut self, now: u64, d: &Durations, auto_start: bool) -> Phase {
        let done = self.phase;
        self.phase = match done {
            Phase::Focus => {
                self.cycle += 1;
                if self.cycle >= d.long_every {
                    self.cycle = 0;
                    Phase::Long
                } else {
                    Phase::Short
                }
            }
            Phase::Short | Phase::Long => Phase::Focus,
        };
        self.remaining_ms = 0;
        if auto_start {
            self.ends_at = now + d.of(self.phase);
            self.status = Status::Running;
        } else {
            self.ends_at = 0;
            self.status = Status::Idle;
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Durations = Durations { focus: 25_000, short: 5_000, long: 15_000, long_every: 2 };

    #[test]
    fn cycle_leads_to_long_break_every_n_focus() {
        let mut p = Pomodoro::default();
        let mut phases = vec![];
        for i in 0..5 {
            p.start(0, &D);
            phases.push(p.phase);
            p.advance(i, &D, false);
        }
        assert_eq!(phases, [Phase::Focus, Phase::Short, Phase::Focus, Phase::Long, Phase::Focus]);
        assert_eq!(p.cycle, 1);
    }

    #[test]
    fn pause_and_resume_keep_remaining_time() {
        let mut p = Pomodoro::default();
        p.start(1_000, &D);
        assert_eq!(p.ends_at, 26_000);
        p.pause(11_000);
        assert_eq!((p.status, p.remaining_ms), (Status::Paused, 15_000));
        p.start(50_000, &D);
        assert_eq!((p.status, p.ends_at), (Status::Running, 65_000));
    }

    #[test]
    fn due_only_when_running_and_past_end() {
        let mut p = Pomodoro::default();
        assert!(!p.is_due(u64::MAX));
        p.start(0, &D);
        assert!(!p.is_due(24_999));
        assert!(p.is_due(25_000));
        p.pause(1);
        assert!(!p.is_due(u64::MAX));
    }

    #[test]
    fn auto_start_runs_the_next_phase() {
        let mut p = Pomodoro::default();
        p.start(0, &D);
        assert_eq!(p.advance(25_000, &D, true), Phase::Focus);
        assert_eq!((p.status, p.phase, p.ends_at), (Status::Running, Phase::Short, 30_000));
    }

    #[test]
    fn reset_goes_back_to_a_fresh_focus() {
        let mut p = Pomodoro::default();
        p.start(0, &D);
        p.advance(1, &D, false);
        p.reset();
        assert_eq!(p, Pomodoro::default());
    }
}
