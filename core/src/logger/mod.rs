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

mod builder;
pub use self::builder::DispatchBuilder;
pub use self::builder::LoggerBuilder;
pub use self::builder::builder;

mod log_impl;
pub use self::log_impl::Logger;
pub use self::log_impl::default_logger;
pub use self::log_impl::set_default_logger;
