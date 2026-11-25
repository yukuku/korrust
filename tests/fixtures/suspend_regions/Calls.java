import java.util.ArrayList;
import java.util.List;
import kotlin.ResultKt;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static final class Done implements Continuation<Object> {
        int count;
        Object result;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object value) { count++; result = value; }
        Object success() { check(count == 1); ResultKt.throwOnFailure(result); return result; }
        void failure(Throwable expected) {
            check(count == 1);
            try { ResultKt.throwOnFailure(result); throw new AssertionError(); }
            catch (Throwable actual) { check(actual == expected); }
        }
    }
    static final class Step implements Function2<String, Continuation<? super String>, Object> {
        final List<String> inputs = new ArrayList<>();
        Continuation<? super String> pending;
        boolean immediate;
        RuntimeException thrown;
        public Object invoke(String value, Continuation<? super String> completion) {
            inputs.add(value);
            if (thrown != null) { var error = thrown; thrown = null; throw error; }
            if (immediate) return "immediate";
            check(pending == null);
            pending = completion;
            return SUSPENDED;
        }
        void resume(Object value) {
            var completion = pending;
            check(completion != null);
            pending = null;
            completion.resumeWith(value);
        }
        void fail(Throwable error) { resume(ResultKt.createFailure(error)); }
    }
    static void catches() {
        for (int mode = 0; mode < 4; mode++) {
            Step step = new Step(); Done done = new Done(); Trace trace = new Trace();
            check(RegionsKt.caught(step, trace, done) == SUSPENDED);
            check(trace.getText().equals("") && done.count == 0);
            if (mode == 0) {
                step.resume("body-result");
                check(done.success().equals("body-result") && trace.getText().equals("F"));
            } else if (mode == 1) {
                var error = new IllegalArgumentException("unmatched"); step.fail(error);
                done.failure(error); check(trace.getText().equals("F"));
            } else {
                step.fail(new IllegalStateException("caught"));
                check(step.inputs.equals(List.of("body", "catch")) && done.count == 0);
                check(trace.getText().equals("C"));
                if (mode == 2) { step.resume("catch-result"); check(done.success().equals("catch-result")); }
                else { var error = new IllegalStateException("catch-failure"); step.fail(error); done.failure(error); }
                check(trace.getText().equals("CF"));
            }
        }
        Step step = new Step(); step.immediate = true; step.thrown = new IllegalStateException("immediate");
        Trace trace = new Trace(); Done done = new Done();
        check(RegionsKt.caught(step, trace, done).equals("immediate"));
        check(trace.getText().equals("CF") && done.count == 0);
    }
    static void returns() {
        Step step = new Step(); Done done = new Done(); Trace trace = new Trace();
        check(RegionsKt.returning(step, trace, done) == SUSPENDED);
        check(trace.getText().equals("B") && done.count == 0);
        step.resume("discarded");
        check(done.success().equals(9000000001L) && trace.getText().equals("BA"));
        Step immediate = new Step(); immediate.immediate = true; Trace fastTrace = new Trace();
        check(RegionsKt.returning(immediate, fastTrace, new Done()).equals(9000000001L));
        check(fastTrace.getText().equals("BA"));
        Step failed = new Step(); Done failure = new Done(); Trace failTrace = new Trace();
        check(RegionsKt.returning(failed, failTrace, failure) == SUSPENDED);
        var error = new IllegalStateException("return-finally"); failed.fail(error); failure.failure(error);
        check(failTrace.getText().equals("B"));
    }
    static void expressions() {
        for (int mode = 0; mode < 4; mode++) {
            Step step = new Step(); Done done = new Done(); Trace trace = new Trace();
            check(RegionsKt.expression(step, trace, done) == SUSPENDED);
            var original = new IllegalArgumentException("body-error");
            if (mode >= 2) step.fail(original); else step.resume("saved-result");
            check(done.count == 0 && trace.getText().equals("B"));
            check(step.inputs.equals(List.of("body", "finally")));
            if (mode % 2 == 1) {
                var override = new IllegalStateException("finally-error"); step.fail(override); done.failure(override);
                check(trace.getText().equals("B"));
            } else {
                step.resume("discarded");
                if (mode >= 2) done.failure(original); else check(done.success().equals("saved-result"));
                check(trace.getText().equals("BA"));
            }
        }
    }
    static void nested() {
        Step step = new Step(); Done done = new Done(); Trace trace = new Trace();
        check(RegionsKt.recovering(step, trace, done) == SUSPENDED);
        check(trace.getText().equals("N") && done.count == 0);
        step.fail(new IllegalStateException("cancel-return"));
        check(trace.getText().equals("NC") && done.count == 0);
        check(step.inputs.equals(List.of("nested", "recovered")));
        step.resume("discarded");
        check(done.success().equals("original") && trace.getText().equals("NCF"));
        Step override = new Step(); Done overrideDone = new Done(); Trace overrideTrace = new Trace();
        check(RegionsKt.recovering(override, overrideTrace, overrideDone) == SUSPENDED);
        override.resume("discarded");
        check(overrideDone.success().equals("override") && overrideTrace.getText().equals("N"));
    }
    static void loops() {
        Step step = new Step(); Done done = new Done(); Trace trace = new Trace();
        check(RegionsKt.loop(step, trace, done) == SUSPENDED);
        for (int index = 0; index < 4; index++) {
            check(done.count == 0);
            step.resume("discarded");
        }
        check(step.inputs.equals(List.of("body", "finally", "body", "finally")));
        check(done.success().equals("BABA") && trace.getText().equals("BABA"));
    }
    public static void main(String[] args) {
        catches(); returns(); expressions(); nested(); loops();
        System.out.println("OK");
    }
}
