/* Copyright (C) 2020 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Animation details */

use std::time::Duration;

/// The keyboard should hide after this has elapsed to prevent flickering.
pub const HIDING_TIMEOUT: Duration = Duration::from_millis(200);

/// The outwardly visible state of visibility
#[derive(PartialEq, Debug, Clone)]
pub enum Outcome {
    Visible,
    Hidden,
}
