package suspension;

import java.util.Objects;
import kotlin.ResultKt;
import kotlin.Unit;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function1;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static final class Completion implements Continuation<Object> {
        Object result;
        int count;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) { this.result = result; count++; }
    }
    static final class Step<A, R> implements Function2<A, Continuation<? super R>, Object> {
        A input;
        Continuation<? super R> continuation;
        int count;
        public Object invoke(A input, Continuation<? super R> continuation) {
            this.input = input;
            this.continuation = continuation;
            count++;
            return SUSPENDED;
        }
    }
    static final class Step0<R> implements Function1<Continuation<? super R>, Object> {
        Continuation<? super R> continuation;
        public Object invoke(Continuation<? super R> continuation) { this.continuation = continuation; return SUSPENDED; }
    }
    static void expect(Object expected, Object actual) {
        if (!Objects.equals(expected, actual)) throw new AssertionError(expected + " != " + actual);
    }
    static void failureIdentity(Throwable expected, Completion completion) {
        expect(1, completion.count);
        try { ResultKt.throwOnFailure(completion.result); throw new AssertionError("Missing failure"); }
        catch (Throwable actual) { if (actual != expected) throw new AssertionError("Failure identity changed", actual); }
    }
    static void delayedChain() {
        ModelsKt.setTrace("");
        Step<Integer, String> first = new Step<>();
        Step<String, Long> second = new Step<>();
        Completion completion = new Completion();
        expect(SUSPENDED, ModelsKt.combine(4, first, second, completion));
        expect(4, first.input); expect(false, first.input == Integer.valueOf(4)); expect(1, first.count); expect(0, second.count); expect(0, completion.count); expect("", ModelsKt.getTrace());
        first.continuation.resumeWith("text");
        expect("text-8", second.input); expect(1, second.count); expect(0, completion.count); expect("a", ModelsKt.getTrace());
        second.continuation.resumeWith(42L);
        expect(1, completion.count); expect("text-8:42", completion.result); expect("ab", ModelsKt.getTrace());
        Step<Integer, Integer> nested = new Step<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.outer(3, nested, completion)); expect(6, nested.input); expect(0, completion.count);
        nested.continuation.resumeWith(5);
        expect(1, completion.count); expect(16, completion.result); expect(false, completion.result == Integer.valueOf(16));
        Step<String, String> imported = new Step<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.imported("source", imported, completion)); expect("source", imported.input); expect(0, completion.count);
        imported.continuation.resumeWith("value"); expect(1, completion.count); expect("[valuedep]", completion.result);
        Step<Integer, Integer> mutation = new Step<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.mutation(mutation, completion)); expect(3, mutation.input);
        mutation.continuation.resumeWith(5); expect(1, completion.count); expect(45, completion.result);
    }
    static void scalarResults() {
        expect(11, ModelsKt.valueNeighbor(11));
        ModelsKt.setTrace("");
        Step<Double, Double> wide = new Step<>(); Completion completion = new Completion();
        expect(SUSPENDED, ModelsKt.wide(8L, 0.5, wide, completion)); expect(8.5, wide.input);
        wide.continuation.resumeWith(2.25); expect(1, completion.count); expect(10.75, completion.result);
        Step0<String> nullable = new Step0<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.nullable(nullable, completion)); nullable.continuation.resumeWith(null);
        expect(1, completion.count); expect(null, completion.result); expect("n", ModelsKt.getTrace());
        Step0<Unit> unit = new Step0<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.unit(unit, completion)); expect("n", ModelsKt.getTrace());
        unit.continuation.resumeWith(Unit.INSTANCE); expect(1, completion.count); expect(Unit.INSTANCE, completion.result); expect("nu", ModelsKt.getTrace());
        completion = new Completion();
        expect("value-4:9", ModelsKt.combine(2, (input, continuation) -> "value", (input, continuation) -> 9L, completion));
        expect(0, completion.count); expect("nuab", ModelsKt.getTrace());
        expect("now", ModelsKt.immediate("now", null));
        completion = new Completion();
        expect(Unit.INSTANCE, ModelsKt.unitTail(continuation -> Unit.INSTANCE, completion)); expect(0, completion.count);
        Step0<Unit> tail = new Step0<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.unitTail(tail, completion)); tail.continuation.resumeWith(Unit.INSTANCE);
        expect(Unit.INSTANCE, completion.result); expect(1, completion.count);
    }
    static void failures() {
        RuntimeException failure = new RuntimeException("identity");
        Completion completion = new Completion();
        try { ModelsKt.leaf(1, (input, continuation) -> { throw failure; }, completion); throw new AssertionError("Missing immediate failure"); }
        catch (RuntimeException actual) { if (actual != failure) throw new AssertionError("Immediate identity changed", actual); }
        expect(0, completion.count);
        Step<Integer, Integer> step = new Step<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.outer(2, step, completion));
        step.continuation.resumeWith(ResultKt.createFailure(failure)); failureIdentity(failure, completion);
        ModelsKt.setTrace(""); Step0<String> after = new Step0<>(); completion = new Completion();
        expect(SUSPENDED, ModelsKt.throwAfter(after, failure, completion));
        after.continuation.resumeWith("after"); expect("after", ModelsKt.getTrace()); failureIdentity(failure, completion);
        completion = new Completion();
        try { ModelsKt.throwAfter(continuation -> "sync", failure, completion); throw new AssertionError("Missing synchronous post-call failure"); }
        catch (RuntimeException actual) { if (actual != failure) throw new AssertionError("Post-call identity changed", actual); }
        expect(0, completion.count);
    }
    static void privateInterop() {
        Step0<String> step = new Step0<>(); Completion completion = new Completion();
        expect(SUSPENDED, ModelsKt.privatePath(step, completion)); expect(0, completion.count);
        step.continuation.resumeWith("value"); expect(1, completion.count); expect("valuehidden!", completion.result);
    }
    static void boxingPositions() {
        Step0<Unit> step = new Step0<>(); Completion completion = new Completion();
        expect(SUSPENDED, ModelsKt.boxingPositions(7, step, completion));
        expect(0, completion.count); step.continuation.resumeWith(Unit.INSTANCE); expect(1, completion.count);
        Object[] values = (Object[]) completion.result;
        expect(4, values.length);
        for (Object value : values) expect(7, value);
        expect(false, values[0] == Integer.valueOf(7));
        expect(false, values[1] == Integer.valueOf(7));
        expect(false, values[2] == Integer.valueOf(7));
        expect(true, values[3] == Integer.valueOf(7));
    }
    public static void main(String[] args) {
        switch (args[0]) {
            case "delayed": delayedChain(); break;
            case "scalars": scalarResults(); break;
            case "failures": failures(); break;
            case "private": privateInterop(); break;
            case "boxing": boxingPositions(); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("OK");
    }
}
