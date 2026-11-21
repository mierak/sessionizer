use std::fmt::{self, Display};
use std::rc::Rc;

use skim::matcher::Matcher;
use skim::{CaseMatching, MatchEngine, MatchEngineFactory, MatchResult, SkimItem, SkimOptions};

use crate::prompt_item::PromptItem;

const RUNNING_SESSION_BONUS: i32 = 1 << 30;

pub fn running_sessions_first_matcher(options: &SkimOptions) -> Matcher {
    let (engine_factory, rank_builder) = Matcher::create_engine_factory_with_builder(options);
    Matcher::builder(Rc::new(RunningSessionsFirstFactory(engine_factory)))
        .case(options.case)
        .rank_builder(rank_builder)
        .build()
}

struct RunningSessionsFirstFactory(Rc<dyn MatchEngineFactory>);

impl MatchEngineFactory for RunningSessionsFirstFactory {
    fn create_engine_with_case(&self, query: &str, case: CaseMatching) -> Box<dyn MatchEngine> {
        Box::new(RunningSessionsFirstEngine(self.0.create_engine_with_case(query, case)))
    }
}

struct RunningSessionsFirstEngine(Box<dyn MatchEngine>);

impl MatchEngine for RunningSessionsFirstEngine {
    fn match_item(&self, item: &dyn SkimItem) -> Option<MatchResult> {
        let mut result = self.0.match_item(item)?;
        let is_running = item
            .as_any()
            .downcast_ref::<PromptItem>()
            .is_some_and(PromptItem::is_running);
        if is_running {
            result.rank.score = result.rank.score.saturating_add(RUNNING_SESSION_BONUS);
        }
        Some(result)
    }
}

impl Display for RunningSessionsFirstEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(RunningSessionsFirst: {})", self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use skim::matcher::Matcher;
    use skim::{CaseMatching, MatchEngine, MatchEngineFactory, SkimOptions};

    use super::RunningSessionsFirstFactory;
    use crate::prompt_item::PromptItem;
    use crate::tmux::SessionStats;

    fn item(name: &str, running: bool) -> PromptItem {
        let mut item = PromptItem::new(name.to_owned(), "/".to_owned().try_into().unwrap());
        if running {
            item.stats = Some(SessionStats {
                id: format!("${name}"),
                window_count: 1,
                attached: false,
                current: false,
            });
        }
        item
    }

    fn engine(query: &str) -> Box<dyn MatchEngine> {
        let (inner, _) = Matcher::create_engine_factory_with_builder(&SkimOptions::default());
        RunningSessionsFirstFactory(inner).create_engine_with_case(query, CaseMatching::default())
    }

    fn score(engine: &dyn MatchEngine, item: &PromptItem) -> i32 {
        engine.match_item(item).unwrap().rank.score
    }

    #[test]
    fn running_session_outranks_better_match_that_is_not_running() {
        let engine = engine("rmpc");

        let weak_match_running = score(engine.as_ref(), &item("x-r-m-p-c", true));
        let exact_match_not_running = score(engine.as_ref(), &item("rmpc", false));

        assert!(weak_match_running > exact_match_not_running);
    }

    #[test]
    fn match_quality_still_orders_sessions_within_each_group() {
        let engine = engine("rmpc");

        assert!(score(engine.as_ref(), &item("rmpc", true)) > score(engine.as_ref(), &item("x-r-m-p-c", true)));
        assert!(score(engine.as_ref(), &item("rmpc", false)) > score(engine.as_ref(), &item("x-r-m-p-c", false)));
    }

    #[test]
    fn items_that_do_not_match_stay_filtered_out() {
        let engine = engine("rmpc");

        assert!(engine.match_item(&item("alpha", true)).is_none());
    }
}
