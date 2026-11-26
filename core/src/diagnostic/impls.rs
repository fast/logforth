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

use std::sync::Arc;

use crate::Diagnostic;
use crate::Error;
use crate::kv;

// Registration boundary: long-lived pipeline storage requires `'static`, even though `Diagnostic`
// itself omits `'static` so per-call contexts can borrow stack data.
impl<T: Diagnostic + 'static> From<T> for Box<dyn Diagnostic> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

// Reference and shared-ownership forwarding, matching the core extension trait forwarding model
// (with `&T` generalized from `&'static T` to any lifetime).
impl<T: Diagnostic + ?Sized> Diagnostic for &T {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for Arc<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

// Homogeneous sequence composition.
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

// `Box<dyn Diagnostic>` cannot implement `Diagnostic` directly because `From<T> for Box<dyn
// Diagnostic>` above would overlap with `core`'s reflexive `From<T> for T`. Provide sequence
// implementations for boxed trait objects directly so owned heterogeneous lists (such as `Dispatch`
// and custom routers) compose on equal footing with `[T]` and `Vec<T>`.
impl Diagnostic for [Box<dyn Diagnostic>] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for diag in self {
            diag.as_ref().visit(visitor)?;
        }
        Ok(())
    }
}

impl<const N: usize> Diagnostic for [Box<dyn Diagnostic>; N] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

impl Diagnostic for Vec<Box<dyn Diagnostic>> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

// Conditional and heterogeneous product composition (`Option<T>`, `()`, and 1..=12 tuples).
impl<T: Diagnostic> Diagnostic for Option<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        if let Some(diag) = self {
            diag.visit(visitor)?;
        }
        Ok(())
    }
}

impl Diagnostic for () {
    fn visit(&self, _: &mut dyn kv::Visitor) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! impl_diagnostic_tuple {
    ($($name:ident)+) => {
        #[allow(non_snake_case)]
        impl<$($name: Diagnostic),+> Diagnostic for ($($name,)+) {
            fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
                let ($($name,)+) = self;
                $(
                    $name.visit(visitor)?;
                )+
                Ok(())
            }
        }
    };
}

impl_diagnostic_tuple!(T0);
impl_diagnostic_tuple!(T0 T1);
impl_diagnostic_tuple!(T0 T1 T2);
impl_diagnostic_tuple!(T0 T1 T2 T3);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6 T7);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6 T7 T8);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10);
impl_diagnostic_tuple!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11);
