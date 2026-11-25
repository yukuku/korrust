package suspendrefs;

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
    static void check(boolean condition) { if (!condition) throw new AssertionError(); }
    static final class Done implements Continuation<Object> {
        Object result;
        int count;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object value) { result = value; count++; }
        Object success() { check(count == 1); ResultKt.throwOnFailure(result); return result; }
    }
    static final class Step implements Function2<String, Continuation<? super String>, Object> {
        final ArrayList<String> inputs = new ArrayList<>();
        Continuation<? super String> pending;
        public Object invoke(String value, Continuation<? super String> continuation) {
            inputs.add(value);
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
    static void top() {
        Step step = new Step(), other = new Step();
        Done done = new Done(), otherDone = new Done();
        var reference = ModelsKt.topReference();
        check(reference instanceof kotlin.coroutines.jvm.internal.SuspendFunction);
        check(reference.invoke("T", step, done) == SUSPENDED);
        check(reference.invoke("U", other, otherDone) == SUSPENDED);
        other.resume("other");
        check("other".equals(otherDone.success()) && done.count == 0);
        check(done.count == 0 && step.inputs.equals(List.of("T")));
        step.resume("top");
        check("top".equals(done.success()));
        check(ModelsKt.ordinaryReference().invoke(5, new Done()).equals(12));
        check(ModelsKt.scalarReference().invoke("abc", new Done()).equals(3));
        check(ModelsKt.unitReference().invoke("unit", new Done()) == kotlin.Unit.INSTANCE);
    }
    static void members() {
        for (int kind = 0; kind < 5; kind++) {
            Step step = new Step();
            Worker worker = new Worker("P", step);
            Holder holder = new Holder(worker);
            Done done = new Done();
            Object value;
            if (kind == 1) {
                value = ModelsKt.unboundReference().invoke(worker, "x", done);
            } else {
                Function2<String, Continuation<? super String>, Object> reference = switch (kind) {
                    case 0 -> ModelsKt.boundReference(worker);
                    case 2 -> ModelsKt.interfaceReference(worker);
                    case 3 -> ModelsKt.evaluatedReference(holder);
                    default -> ModelsKt.nullableReference(worker);
                };
                check(reference instanceof kotlin.coroutines.jvm.internal.SuspendFunction);
                check(holder.getVisits() == (kind == 3 ? 1 : 0));
                value = reference.invoke("x", done);
                check(holder.getVisits() == (kind == 3 ? 1 : 0));
            }
            check(value == SUSPENDED && done.count == 0);
            check(step.inputs.equals(List.of("Px")));
            step.resume("first");
            check(done.count == 0 && step.inputs.equals(List.of("Px", "first")));
            step.resume("last");
            check("Plast".equals(done.success()));
            check(holder.getVisits() == (kind == 3 ? 1 : 0));
        }
    }
    static void failures() {
        for (boolean second : new boolean[] {false, true}) {
            Step step = new Step();
            Done done = new Done();
            check(ModelsKt.boundReference(new Worker("E", step)).invoke("x", done) == SUSPENDED);
            if (second) step.resume("first");
            IllegalArgumentException failure = new IllegalArgumentException("same");
            step.resume(ResultKt.createFailure(failure));
            check(done.count == 1);
            try {
                ResultKt.throwOnFailure(done.result);
                throw new AssertionError();
            } catch (IllegalArgumentException actual) {
                check(actual == failure);
            }
        }
    }
    public static void main(String[] args) {
        switch (args[0]) {
            case "top" -> top();
            case "members" -> members();
            case "failures" -> failures();
            default -> throw new AssertionError(args[0]);
        }
        System.out.println("OK");
    }
}
