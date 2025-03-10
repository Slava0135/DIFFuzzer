/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{cell::RefCell, rc::Rc};

use log::debug;

use crate::{config::Config, fuzzing::observer::dash::DashObserver};

use dash::{FileDiff, HasherOptions, get_diff};

pub struct DashObjective {
    fst_observer: Rc<RefCell<DashObserver>>,
    snd_observer: Rc<RefCell<DashObserver>>,
    enabled: bool,
    hasher_options: HasherOptions,
}

impl DashObjective {
    pub fn new(
        config: &Config,
        fst_observer: Rc<RefCell<DashObserver>>,
        snd_observer: Rc<RefCell<DashObserver>>,
    ) -> Self {
        Self {
            enabled: config.dash.enabled,
            hasher_options: Default::default(),
            fst_observer,
            snd_observer,
        }
    }

    pub fn is_interesting(&self) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if !self.enabled {
            return Ok(false);
        }

        Ok(self.fst_observer.borrow().hash() != self.snd_observer.borrow().hash())
    }

    pub fn get_diff(&self) -> Vec<FileDiff> {
        get_diff(
            &self.fst_observer.borrow().fs_state(),
            &self.snd_observer.borrow().fs_state(),
            &self.fst_observer.borrow().fs_internal(),
            &self.snd_observer.borrow().fs_internal(),
            &self.hasher_options,
        )
    }
}
