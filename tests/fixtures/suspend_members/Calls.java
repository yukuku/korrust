package members;

import java.util.ArrayList;
import kotlin.ResultKt;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static final class Completion implements Continuation<Object> {
        Object value;
        int count;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) { value = result; count++; }
        Object success() { check(count == 1); ResultKt.throwOnFailure(value); return value; }
    }
    static final class Step implements Function2<String, Continuation<? super String>, Object> {
        final ArrayList<String> inputs = new ArrayList<>();
        Continuation<? super String> pending;
        final boolean delayed;
        Step(boolean delayed) { this.delayed = delayed; }
        public Object invoke(String input, Continuation<? super String> continuation) {
            inputs.add(input);
            if (!delayed) return input + "!";
            check(pending == null);
            pending = continuation;
            return SUSPENDED;
        }
        void resume(Object value) {
            var continuation = pending;
            check(continuation != null);
            pending = null;
            continuation.resumeWith(value);
        }
    }
    static void immediate() throws Exception {
        Completion done = new Completion();
        Immediate reader = new Immediate("I");
        check("Ix".equals(reader.read("x", done)));
        check("Iy".equals(reader.retry("y", done)));
        check("In".equals(reader.read("n", null)));
        check("Im".equals(reader.retry("m", null)));
        var helper = Boundary.class.getDeclaredMethod("retry$suspendImpl", Boundary.class, String.class, Continuation.class);
        check(helper.isSynthetic());
        check("Ih".equals(helper.invoke(null, new Object[] {reader, "h", null})));
        check("Iz".equals(ModelsKt.dispatch(reader, "z", done)));
        Step callback = new Step(false);
        Reader stateful = new Reader("A", callback);
        check("AAx:!!1".equals(ModelsKt.concrete(stateful, "x", done)));
        check(stateful.getVisits() == 1 && done.count == 0);
        check(callback.inputs.equals(java.util.List.of("Ax:", "Ax:!")));
    }
    static void delayed() {
        Step first = new Step(true), second = new Step(true);
        Reader a = new Reader("A", first), b = new Reader("B", second);
        Completion aDone = new Completion(), bDone = new Completion();
        check(ModelsKt.dispatch(a, "x", aDone) == SUSPENDED);
        check(ModelsKt.concrete(b, "y", bDone) == SUSPENDED);
        check(a.getVisits() == 1 && b.getVisits() == 1);
        check(first.inputs.equals(java.util.List.of("Ax:")));
        check(second.inputs.equals(java.util.List.of("By:")));
        second.resume("b1");
        first.resume("a1");
        check(aDone.count == 0 && bDone.count == 0);
        check(first.inputs.equals(java.util.List.of("Ax:", "a1")));
        check(second.inputs.equals(java.util.List.of("By:", "b1")));
        first.resume("a2");
        second.resume("b2");
        check("Aa21".equals(aDone.success()));
        check("Bb21".equals(bDone.success()));
    }
    static void failures() {
        for (boolean atSecond : new boolean[] {false, true}) {
            Step callback = new Step(true);
            Reader reader = new Reader("E", callback);
            Completion done = new Completion();
            check(ModelsKt.dispatch(reader, "x", done) == SUSPENDED);
            if (atSecond) callback.resume("first");
            IllegalStateException failure = new IllegalStateException("same");
            callback.resume(ResultKt.createFailure(failure));
            check(done.count == 1);
            try { done.success(); throw new AssertionError(); } catch (IllegalStateException actual) { check(actual == failure); }
            check(reader.getVisits() == 1);
        }
        IllegalArgumentException failure = new IllegalArgumentException("immediate");
        Reader reader = new Reader("E", (value, continuation) -> { throw failure; });
        Completion done = new Completion();
        try { ModelsKt.dispatch(reader, "x", done); throw new AssertionError(); } catch (IllegalArgumentException actual) { check(actual == failure); }
        check(done.count == 0);
    }
    static void scalars() {
        final Continuation<?>[] pending = new Continuation<?>[1];
        Completion done = new Completion();
        Scalars scalar = new Scalars(100L);
        check(scalar.combine(20L, 3.5, (value, continuation) -> { check(value == 20L); pending[0] = continuation; return SUSPENDED; }, done) == SUSPENDED);
        pending[0].resumeWith(40L);
        check(done.success().equals(143L));
    }
    static void dependency() {
        final Continuation<?>[] pending = new Continuation<?>[1];
        Completion done = new Completion();
        suspend_dependency.External external = (input, continuation) -> { check(input.equals("x")); pending[0] = continuation; return SUSPENDED; };
        check(ModelsKt.dependency(external, "x", done) == SUSPENDED);
        pending[0].resumeWith(null);
        check(done.success() == null);
        Step callback = new Step(true);
        ExternalReader reader = new ExternalReader(callback);
        Completion sourceDone = new Completion();
        check(ModelsKt.dependency(reader, "source", sourceDone) == SUSPENDED);
        callback.resume("compiled-interface");
        check(sourceDone.success().equals("compiled-interface"));
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "immediate": immediate(); break;
            case "delayed": delayed(); break;
            case "failures": failures(); break;
            case "scalars": scalars(); break;
            case "dependency": dependency(); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("OK");
    }
}
