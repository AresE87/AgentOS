use serde::{Deserialize, Serialize};

/// R135 — Education vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub subject: String,
    pub level: String,
    pub lessons: Vec<Lesson>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub title: String,
    pub content: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentProgress {
    pub student_id: String,
    pub course_id: String,
    pub completed_lessons: Vec<String>,
    pub quiz_scores: Vec<f64>,
    pub overall_grade: Option<f64>,
}

pub struct EducationAssistant {
    courses: Vec<Course>,
    progress: Vec<StudentProgress>,
    next_id: u64,
}

impl EducationAssistant {
    pub fn new() -> Self {
        Self {
            courses: Vec::new(),
            progress: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new course.
    pub fn create_course(
        &mut self,
        title: String,
        subject: String,
        level: String,
        lesson_titles: Vec<String>,
    ) -> Course {
        let lessons: Vec<Lesson> = lesson_titles
            .into_iter()
            .enumerate()
            .map(|(i, t)| Lesson {
                id: format!("lesson_{}_{}", self.next_id, i + 1),
                title: t,
                content: String::new(),
                order: (i + 1) as u32,
            })
            .collect();

        let course = Course {
            id: format!("course_{}", self.next_id),
            title,
            subject,
            level,
            lessons,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.courses.push(course.clone());
        course
    }

    /// Generate a quiz for a course.
    pub fn generate_quiz(
        &self,
        course_id: &str,
        num_questions: u32,
    ) -> Result<serde_json::Value, String> {
        let course = self
            .courses
            .iter()
            .find(|c| c.id == course_id)
            .ok_or_else(|| format!("Course not found: {}", course_id))?;

        let questions: Vec<serde_json::Value> = (1..=num_questions)
            .map(|i| {
                let lesson_idx = ((i - 1) as usize) % course.lessons.len().max(1);
                let lesson_title = course
                    .lessons
                    .get(lesson_idx)
                    .map(|l| l.title.as_str())
                    .unwrap_or("General");
                serde_json::json!({
                    "question_id": i,
                    "topic": lesson_title,
                    "question": format!("Question {} about {}", i, lesson_title),
                    "options": ["A", "B", "C", "D"],
                    "correct_answer": "A",
                })
            })
            .collect();

        Ok(serde_json::json!({
            "course_id": course.id,
            "course_title": course.title,
            "num_questions": num_questions,
            "questions": questions,
        }))
    }

    /// Grade a student's answer.
    pub fn grade_answer(
        &mut self,
        student_id: &str,
        course_id: &str,
        score: f64,
    ) -> serde_json::Value {
        let progress = self
            .progress
            .iter_mut()
            .find(|p| p.student_id == student_id && p.course_id == course_id);

        match progress {
            Some(p) => {
                p.quiz_scores.push(score);
                let avg = p.quiz_scores.iter().sum::<f64>() / p.quiz_scores.len() as f64;
                p.overall_grade = Some(avg);
                serde_json::json!({
                    "student_id": student_id,
                    "course_id": course_id,
                    "score": score,
                    "average_score": avg,
                    "total_quizzes": p.quiz_scores.len(),
                })
            }
            None => {
                self.progress.push(StudentProgress {
                    student_id: student_id.to_string(),
                    course_id: course_id.to_string(),
                    completed_lessons: Vec::new(),
                    quiz_scores: vec![score],
                    overall_grade: Some(score),
                });
                serde_json::json!({
                    "student_id": student_id,
                    "course_id": course_id,
                    "score": score,
                    "average_score": score,
                    "total_quizzes": 1,
                })
            }
        }
    }

    /// Track student progress in a course.
    pub fn track_progress(&self, student_id: &str, course_id: &str) -> serde_json::Value {
        let course = self.courses.iter().find(|c| c.id == course_id);
        let progress = self
            .progress
            .iter()
            .find(|p| p.student_id == student_id && p.course_id == course_id);

        let total_lessons = course.map(|c| c.lessons.len()).unwrap_or(0);
        let completed = progress.map(|p| p.completed_lessons.len()).unwrap_or(0);
        let pct = if total_lessons > 0 {
            (completed as f64 / total_lessons as f64) * 100.0
        } else {
            0.0
        };

        serde_json::json!({
            "student_id": student_id,
            "course_id": course_id,
            "total_lessons": total_lessons,
            "completed_lessons": completed,
            "completion_pct": pct,
            "average_quiz_score": progress.and_then(|p| p.overall_grade),
            "total_quizzes_taken": progress.map(|p| p.quiz_scores.len()).unwrap_or(0),
        })
    }
}
