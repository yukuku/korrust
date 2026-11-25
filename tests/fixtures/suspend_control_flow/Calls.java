import java.util.*;
import kotlin.ResultKt;
import kotlin.Unit;
import kotlin.coroutines.*;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.*;
public class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static class Done implements Continuation<Object> {
        int count; Object result;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object value) { count++; result = value; }
    }
    static class Step implements Function2<Integer, Continuation<? super Integer>, Object> {
        final List<Integer> calls = new ArrayList<>();
        Continuation<? super Integer> pending;
        final boolean immediate;
        Step(boolean immediate) { this.immediate = immediate; }
        public Object invoke(Integer value, Continuation<? super Integer> completion) {
            calls.add(value); pending = completion;
            return immediate ? value * 2 : SUSPENDED;
        }
        void finish(int... expected) {
            for (int i = 0; i < expected.length; i++) {
                check(calls.size() == i + 1 && calls.get(i) == expected[i]);
                pending.resumeWith(expected[i] * 2);
            }
            check(calls.size() == expected.length);
        }
    }
    public static void main(String[] args) {
        for (boolean quick : new boolean[] {false, true}) {
            for (int mode : new int[] {-1, 0, 1}) {
                Step step = new Step(quick); Done done = new Done();
                Object result = FlowKt.choose(mode, step, done);
                int expected = mode == -1 ? 12 : mode == 0 ? 47 : 48;
                if (mode == -1 || quick) {
                    check(result.equals(expected) && done.count == 0);
                } else {
                    check(result == SUSPENDED && done.count == 0);
                    if (mode == 0) step.finish(10, 20); else step.finish(20);
                    check(done.count == 1 && done.result.equals(expected));
                }
                check(step.calls.equals(mode == -1 ? List.of() : mode == 0 ? List.of(10, 20) : List.of(20)));
            }
            for (int limit : new int[] {0, 4}) {
                Step step = new Step(quick); Done done = new Done();
                Object result = FlowKt.loops(limit, step, done);
                int expected = limit == 0 ? 404 : 579;
                if (quick) check(result.equals(expected) && done.count == 0);
                else {
                    check(result == SUSPENDED && done.count == 0);
                    if (limit == 0) step.finish(100, 102); else step.finish(11, 12, 31, 32, 100, 102);
                    check(done.count == 1 && done.result.equals(expected));
                }
            }
            Step array = new Step(quick); Done done = new Done();
            Object result = FlowKt.array(new int[] {4, 5}, array, done);
            if (quick) check(result.equals(18) && done.count == 0);
            else { check(result == SUSPENDED); array.finish(4, 5); check(done.count == 1 && done.result.equals(18)); }
            Step local = new Step(quick); Done completion = new Done();
            result = FlowKt.localBranch(true, local, completion);
            if (quick) check(result.equals(8) && completion.count == 0);
            else { check(result == SUSPENDED); local.finish(4); check(completion.count == 1 && completion.result.equals(8)); }
            Step skipped = new Step(quick);
            check(FlowKt.localBranch(false, skipped, new Done()).equals(4) && skipped.calls.isEmpty());
        }
        Step failing = new Step(false); Done failed = new Done();
        check(FlowKt.loops(4, failing, failed) == SUSPENDED);
        failing.pending.resumeWith(22);
        RuntimeException error = new RuntimeException("same");
        failing.pending.resumeWith(ResultKt.createFailure(error));
        check(failed.count == 1 && failing.calls.equals(List.of(11, 12)));
        try { ResultKt.throwOnFailure(failed.result); throw new AssertionError(); }
        catch (RuntimeException caught) { check(caught == error); }
        final Continuation<?>[] pending = new Continuation<?>[1];
        Done unit = new Done();
        Function1<Continuation<? super Unit>, Object> delay = completion -> { pending[0] = completion; return SUSPENDED; };
        check(FlowKt.unit(delay, unit) == SUSPENDED && unit.count == 0);
        pending[0].resumeWith(Unit.INSTANCE);
        check(unit.count == 1 && unit.result == Unit.INSTANCE);
        Done immediateUnit = new Done();
        check(FlowKt.unit(completion -> Unit.INSTANCE, immediateUnit) == Unit.INSTANCE && immediateUnit.count == 0);
        for (boolean answer : new boolean[] {false, true}) {
            Done condition = new Done();
            final Continuation<?>[] booleanPending = new Continuation<?>[1];
            Function1<Continuation<? super Boolean>, Object> checkCondition = completion -> { booleanPending[0] = completion; return SUSPENDED; };
            check(FlowKt.condition(false, checkCondition, condition).equals(22) && booleanPending[0] == null);
            check(FlowKt.condition(true, checkCondition, condition) == SUSPENDED && condition.count == 0);
            booleanPending[0].resumeWith(answer);
            check(condition.count == 1 && condition.result.equals(answer ? 11 : 22));
            Done immediate = new Done();
            check(FlowKt.condition(true, completion -> answer, immediate).equals(answer ? 11 : 22) && immediate.count == 0);
        }
        System.out.println("OK");
    }
}
