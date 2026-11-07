package cross;

import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import members.Boundary;

public final class CrossCalls {
    static final class Done implements Continuation<Object> {
        Object value;
        int count;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) { value = result; count++; }
    }
    public static void main(String[] args) {
        Continuation<?>[] pending = new Continuation<?>[1];
        Boundary boundary = (input, continuation) -> {
            if (!input.equals("cross")) throw new AssertionError();
            pending[0] = continuation;
            return IntrinsicsKt.getCOROUTINE_SUSPENDED();
        };
        Imported imported = new Imported(boundary);
        Done done = new Done();
        if (CrossKt.call(imported, "cross", done) != IntrinsicsKt.getCOROUTINE_SUSPENDED()) throw new AssertionError();
        if (done.count != 0) throw new AssertionError();
        pending[0].resumeWith("resumed");
        if (done.count != 1 || !done.value.equals("resumed")) throw new AssertionError();
        System.out.println("OK");
    }
}
