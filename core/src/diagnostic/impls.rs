// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Diagnostic;
use crate::Error;
use crate::kv;

impl Diagnostic for () {
    fn visit(&self, _: &mut dyn kv::Visitor) -> Result<(), Error> {
        Ok(())
    }
}

impl<T: Diagnostic> Diagnostic for Option<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        if let Some(diag) = self {
            diag.visit(visitor)?;
        }
        Ok(())
    }
}

impl<T: Diagnostic> Diagnostic for [T] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for diag in self {
            diag.visit(visitor)?;
        }
        Ok(())
    }
}

impl<T: Diagnostic, const N: usize> Diagnostic for [T; N] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

impl<T: Diagnostic> Diagnostic for Vec<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

// `Box<dyn Diagnostic>` cannot implement `Diagnostic` directly without overlapping
// `From<T> for Box<dyn Diagnostic>` and reflexive `From<T> for T`.
impl Diagnostic for [Box<dyn Diagnostic>] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for diag in self {
            diag.as_ref().visit(visitor)?;
        }
        Ok(())
    }
}

impl Diagnostic for Vec<Box<dyn Diagnostic>> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}
