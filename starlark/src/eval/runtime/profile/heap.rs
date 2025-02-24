/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fmt::Debug;

use gazebo::dupe::Dupe;

use crate::values::layout::heap::stacks::AggregateHeapProfileInfo;
use crate::values::Heap;
use crate::values::Value;

#[derive(Copy, Clone, Dupe, Debug)]
pub(crate) enum HeapProfileFormat {
    Summary,
    FlameGraph,
}

pub(crate) struct HeapProfile {
    enabled: bool,
}

impl HeapProfile {
    pub(crate) fn new() -> Self {
        Self { enabled: false }
    }

    pub(crate) fn enable(&mut self) {
        self.enabled = true;
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn record_call_enter<'v>(&self, function: Value<'v>, heap: &'v Heap) {
        if self.enabled {
            heap.record_call_enter(function);
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn record_call_exit<'v>(&self, heap: &'v Heap) {
        if self.enabled {
            heap.record_call_exit();
        }
    }

    // We could expose profile on the Heap, but it's an implementation detail that it works here.
    pub(crate) fn gen(&self, heap: &Heap, format: HeapProfileFormat) -> Option<String> {
        if !self.enabled {
            None
        } else {
            Some(Self::gen_enabled(heap, format))
        }
    }

    pub(crate) fn gen_enabled(heap: &Heap, format: HeapProfileFormat) -> String {
        match format {
            HeapProfileFormat::Summary => Self::write_summarized_heap_profile(heap),
            HeapProfileFormat::FlameGraph => Self::write_flame_heap_profile(heap),
        }
    }

    fn write_flame_heap_profile(heap: &Heap) -> String {
        let stacks = AggregateHeapProfileInfo::collect(heap, None);
        stacks.write()
    }

    fn write_summarized_heap_profile(heap: &Heap) -> String {
        let stacks = AggregateHeapProfileInfo::collect(heap, None);
        stacks.gen_summary_csv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Globals;
    use crate::environment::Module;
    use crate::eval::Evaluator;
    use crate::eval::ProfileMode;
    use crate::syntax::AstModule;
    use crate::syntax::Dialect;
    use crate::values::Value;

    #[test]
    fn test_profiling() -> anyhow::Result<()> {
        // We don't test that the profile looks any particular way, but we do test it doesn't crash
        let ast = AstModule::parse(
            "foo.bzl",
            r#"
def f(x):
    return (x * 5) + 3
y = 8 * 9 + 2
f
"#
            .to_owned(),
            &Dialect::Extended,
        )?;
        let globals = Globals::standard();
        let module = Module::new();
        let mut eval = Evaluator::new(&module);
        eval.enable_profile(&ProfileMode::HeapSummary);
        let f = eval.eval_module(ast, &globals)?;
        // first check module profiling works
        HeapProfile::write_summarized_heap_profile(module.heap());
        HeapProfile::write_flame_heap_profile(module.heap());

        // second check function profiling works
        let module = Module::new();
        let mut eval = Evaluator::new(&module);
        eval.enable_profile(&ProfileMode::HeapSummary);
        eval.eval_function(f, &[Value::new_int(100)], &[])?;
        HeapProfile::write_summarized_heap_profile(module.heap());
        HeapProfile::write_flame_heap_profile(module.heap());

        // finally, check a user can add values into the heap before/after
        let module = Module::new();
        let mut eval = Evaluator::new(&module);
        module.heap().alloc("Thing that goes before");
        eval.enable_profile(&ProfileMode::HeapSummary);
        eval.eval_function(f, &[Value::new_int(100)], &[])?;
        module.heap().alloc("Thing that goes after");
        HeapProfile::write_summarized_heap_profile(module.heap());
        HeapProfile::write_flame_heap_profile(module.heap());

        Ok(())
    }
}
