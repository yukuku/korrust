package suspendcapacity;

import java.util.Objects;
import kotlin.ResultKt;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static final int[] INPUTS = {1, 3, 7, 15, 31, 63, 127, 255};
    static final String TRACE = "abcdefgh";

    static void expect(Object expected, Object actual) {
        if (!Objects.equals(expected, actual)) throw new AssertionError(expected + " != " + actual);
    }

    static final class Completion implements Continuation<Integer> {
        int count;
        Object result;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) {
            count++;
            expect(1, count);
            ResultKt.throwOnFailure(result);
            this.result = result;
        }
    }

    static final class Step implements Function2<Integer, Continuation<? super Integer>, Object> {
        final boolean delayed;
        int count;
        int pending;
        Continuation<? super Integer> continuation;
        Step(boolean delayed) { this.delayed = delayed; }
        public Object invoke(Integer input, Continuation<? super Integer> continuation) {
            expect(INPUTS[count], input);
            expect(TRACE.substring(0, count), CapacityKt.getTrace());
            count++;
            this.pending = input * 2;
            this.continuation = continuation;
            return delayed ? SUSPENDED : pending;
        }
    }

    static void immediate() {
        CapacityKt.setTrace("");
        Step step = new Step(false);
        Completion completion = new Completion();
        expect(1510, CapacityKt.eight(step, completion));
        expect(8, step.count);
        expect(TRACE, CapacityKt.getTrace());
        expect(0, completion.count);
    }

    static void delayed() {
        CapacityKt.setTrace("");
        Step step = new Step(true);
        Completion completion = new Completion();
        if (CapacityKt.eight(step, completion) != SUSPENDED) throw new AssertionError("Missing suspension");
        for (int index = 0; index < 8; index++) {
            expect(index + 1, step.count);
            expect(TRACE.substring(0, index), CapacityKt.getTrace());
            expect(0, completion.count);
            Continuation<? super Integer> continuation = step.continuation;
            step.continuation = null;
            continuation.resumeWith(step.pending);
            expect(TRACE.substring(0, index + 1), CapacityKt.getTrace());
            expect(Math.min(index + 2, 8), step.count);
            expect(index == 7 ? 1 : 0, completion.count);
        }
        expect(1, completion.count);
        expect(1510, completion.result);
    }

    public static void main(String[] args) {
        immediate();
        delayed();
        System.out.println("OK");
    }
}
