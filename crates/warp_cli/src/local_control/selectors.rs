//! CLI argument conversion into shared local-control selectors.
use local_control::protocol::{
    BlockSelector, BlockTarget, ControlError, ErrorCode, PaneSelector, PaneTarget, SessionSelector,
    SessionTarget, TabSelector, TabTarget, TargetSelector, WindowSelector, WindowTarget,
};
use local_control::selection::InstanceSelector;

use crate::local_control::TargetArgs;

pub(super) fn instance_selector(args: TargetArgs) -> InstanceSelector {
    if let Some(instance_id) = args.instance {
        return InstanceSelector::Id(local_control::discovery::InstanceId(instance_id));
    }
    if let Some(pid) = args.pid {
        return InstanceSelector::Pid(pid);
    }
    InstanceSelector::Active
}

pub(super) fn target_selector(args: TargetArgs) -> Result<TargetSelector, ControlError> {
    Ok(TargetSelector {
        window: window_target(&args)?,
        tab: tab_target(&args)?,
        pane: pane_target(&args)?,
        session: session_target(&args)?,
        block: block_target(&args)?,
        ..TargetSelector::default()
    })
}

fn window_target(args: &TargetArgs) -> Result<Option<WindowTarget>, ControlError> {
    if let Some(selector) = args.window.as_deref() {
        return parse_window_selector(selector).map(Some);
    }
    if let Some(id) = args.window_id.as_ref() {
        return Ok(Some(WindowTarget::Id {
            id: WindowSelector(id.clone()),
        }));
    }
    if let Some(index) = args.window_index {
        return Ok(Some(WindowTarget::Index { index }));
    }
    if let Some(title) = args.window_title.as_ref() {
        return Ok(Some(WindowTarget::Title {
            title: title.clone(),
        }));
    }
    Ok(None)
}

fn tab_target(args: &TargetArgs) -> Result<Option<TabTarget>, ControlError> {
    if let Some(selector) = args.tab.as_deref() {
        return parse_tab_selector(selector).map(Some);
    }
    if let Some(id) = args.tab_id.as_ref() {
        return Ok(Some(TabTarget::Id {
            id: TabSelector(id.clone()),
        }));
    }
    if let Some(index) = args.tab_index {
        return Ok(Some(TabTarget::Index { index }));
    }
    if let Some(title) = args.tab_title.as_ref() {
        return Ok(Some(TabTarget::Title {
            title: title.clone(),
        }));
    }
    Ok(None)
}

fn pane_target(args: &TargetArgs) -> Result<Option<PaneTarget>, ControlError> {
    if let Some(selector) = args.pane.as_deref() {
        return parse_pane_selector(selector).map(Some);
    }
    if let Some(id) = args.pane_id.as_ref() {
        return Ok(Some(PaneTarget::Id {
            id: PaneSelector(id.clone()),
        }));
    }
    if let Some(index) = args.pane_index {
        return Ok(Some(PaneTarget::Index { index }));
    }
    Ok(None)
}

fn session_target(args: &TargetArgs) -> Result<Option<SessionTarget>, ControlError> {
    if let Some(selector) = args.session.as_deref() {
        return parse_session_selector(selector).map(Some);
    }
    if let Some(id) = args.session_id.as_ref() {
        return Ok(Some(SessionTarget::Id {
            id: SessionSelector(id.clone()),
        }));
    }
    if let Some(index) = args.session_index {
        return Ok(Some(SessionTarget::Index { index }));
    }
    Ok(None)
}

fn block_target(args: &TargetArgs) -> Result<Option<BlockTarget>, ControlError> {
    if let Some(selector) = args.block.as_deref() {
        return parse_block_selector(selector).map(Some);
    }
    if let Some(id) = args.block_id.as_ref() {
        return Ok(Some(BlockTarget::Id {
            id: BlockSelector(id.clone()),
        }));
    }
    if let Some(index) = args.block_index {
        return Ok(Some(BlockTarget::Index { index }));
    }
    Ok(None)
}

fn parse_window_selector(selector: &str) -> Result<WindowTarget, ControlError> {
    if selector == "active" {
        return Ok(WindowTarget::Active);
    }
    if let Some(id) = selector.strip_prefix("id:") {
        return Ok(WindowTarget::Id {
            id: WindowSelector(id.to_owned()),
        });
    }
    if let Some(index) = selector.strip_prefix("index:") {
        return parse_index(index).map(|index| WindowTarget::Index { index });
    }
    if let Some(title) = selector.strip_prefix("title:") {
        return Ok(WindowTarget::Title {
            title: title.to_owned(),
        });
    }
    Err(invalid_selector("window", selector))
}

fn parse_tab_selector(selector: &str) -> Result<TabTarget, ControlError> {
    if selector == "active" {
        return Ok(TabTarget::Active);
    }
    if let Some(id) = selector.strip_prefix("id:") {
        return Ok(TabTarget::Id {
            id: TabSelector(id.to_owned()),
        });
    }
    if let Some(index) = selector.strip_prefix("index:") {
        return parse_index(index).map(|index| TabTarget::Index { index });
    }
    if let Some(title) = selector.strip_prefix("title:") {
        return Ok(TabTarget::Title {
            title: title.to_owned(),
        });
    }
    Err(invalid_selector("tab", selector))
}

fn parse_pane_selector(selector: &str) -> Result<PaneTarget, ControlError> {
    if selector == "active" {
        return Ok(PaneTarget::Active);
    }
    if let Some(id) = selector.strip_prefix("id:") {
        return Ok(PaneTarget::Id {
            id: PaneSelector(id.to_owned()),
        });
    }
    if let Some(index) = selector.strip_prefix("index:") {
        return parse_index(index).map(|index| PaneTarget::Index { index });
    }
    Err(invalid_selector("pane", selector))
}

fn parse_session_selector(selector: &str) -> Result<SessionTarget, ControlError> {
    if selector == "active" {
        return Ok(SessionTarget::Active);
    }
    if let Some(id) = selector.strip_prefix("id:") {
        return Ok(SessionTarget::Id {
            id: SessionSelector(id.to_owned()),
        });
    }
    if let Some(index) = selector.strip_prefix("index:") {
        return parse_index(index).map(|index| SessionTarget::Index { index });
    }
    Err(invalid_selector("session", selector))
}

fn parse_block_selector(selector: &str) -> Result<BlockTarget, ControlError> {
    if selector == "active" {
        return Ok(BlockTarget::Active);
    }
    if let Some(id) = selector.strip_prefix("id:") {
        return Ok(BlockTarget::Id {
            id: BlockSelector(id.to_owned()),
        });
    }
    if let Some(index) = selector.strip_prefix("index:") {
        return parse_index(index).map(|index| BlockTarget::Index { index });
    }
    Err(invalid_selector("block", selector))
}

fn parse_index(index: &str) -> Result<u32, ControlError> {
    index.parse::<u32>().map_err(|err| {
        ControlError::with_details(
            ErrorCode::InvalidSelector,
            format!("invalid index selector {index}"),
            err.to_string(),
        )
    })
}

fn invalid_selector(family: &str, selector: &str) -> ControlError {
    ControlError::new(
        ErrorCode::InvalidSelector,
        format!("invalid {family} selector {selector}"),
    )
}
