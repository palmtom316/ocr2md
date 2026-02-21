use std::collections::HashMap;

pub type JobId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Retrying,
    Failed,
    Success,
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub id: JobId,
    pub input: String,
    pub state: JobState,
    pub stage: String,
    pub retries: u8,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct Queue {
    next_id: JobId,
    jobs: HashMap<JobId, JobRecord>,
}

impl Queue {
    pub fn enqueue(&mut self, input: impl Into<String>) -> JobId {
        self.next_id += 1;
        let id = self.next_id;
        self.jobs.insert(
            id,
            JobRecord {
                id,
                input: input.into(),
                state: JobState::Queued,
                stage: "queued".to_string(),
                retries: 0,
                error: None,
            },
        );
        id
    }

    pub fn mark_running(&mut self, id: JobId, stage: impl Into<String>) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.state = JobState::Running;
            job.stage = stage.into();
            job.error = None;
        }
    }

    pub fn mark_retrying(&mut self, id: JobId, stage: impl Into<String>, error: impl Into<String>) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.state = JobState::Retrying;
            job.stage = stage.into();
            job.retries = job.retries.saturating_add(1);
            job.error = Some(error.into());
        }
    }

    pub fn mark_failed(&mut self, id: JobId, error: impl Into<String>) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.state = JobState::Failed;
            job.error = Some(error.into());
        }
    }

    pub fn mark_success(&mut self, id: JobId) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.state = JobState::Success;
            job.stage = "done".to_string();
            job.error = None;
        }
    }

    pub fn get(&self, id: JobId) -> Option<&JobRecord> {
        self.jobs.get(&id)
    }

    pub fn get_next_pending(&self) -> Option<JobId> {
        let mut pending: Vec<&JobRecord> = self
            .jobs
            .values()
            .filter(|job| job.state == JobState::Queued || job.state == JobState::Retrying)
            .collect();
        pending.sort_by_key(|job| job.id);
        pending.first().map(|job| job.id)
    }
}
