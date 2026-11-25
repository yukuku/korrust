import together.*;
import kotlin.Unit;
import kotlin.coroutines.*;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function0;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static final class Pause implements Function2<String, Continuation<? super String>, Object> {
        Continuation<? super String> continuation;
        public Object invoke(String value, Continuation<? super String> next) {
            if (!"一".equals(value) || continuation != null) throw new AssertionError(value);
            continuation = next;
            return IntrinsicsKt.getCOROUTINE_SUSPENDED();
        }
    }

    static final class Done implements Continuation<String> {
        int calls;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) {
            if (!"resumed".equals(result) || ++calls != 1) throw new AssertionError(result);
        }
    }

    public static void main(String[] args) {
        if (!"α".equals(ExtendedKt.anonymousEcho().echo("α")))
            throw new AssertionError("inherited generic default");
        Base<String> source = ExtendedKt.source("一");
        Identity identity = new Identity();
        int[] finalized = {0};
        Function0<Unit> after = () -> { finalized[0]++; return Unit.INSTANCE; };
        Done done = new Done();
        Pause pause = new Pause();
        Object skip = ExtendedKt.readIf(false, source, identity, after, pause, done);
        if (!"off".equals(skip) || finalized[0] != 0 || pause.continuation != null)
            throw new AssertionError("disabled branch");
        Object initial = ExtendedKt.readIf(true, source, identity, after, pause, done);
        if (initial != IntrinsicsKt.getCOROUTINE_SUSPENDED() || finalized[0] != 1 || done.calls != 0)
            throw new AssertionError("enabled branch");
        pause.continuation.resumeWith("resumed");
        if (done.calls != 1 || finalized[0] != 1) throw new AssertionError("resumed branch");
        Done resumed = new Done();
        Pause delayed = new Pause();
        Object pending = ExtendedKt.inlineAfterResume(source, identity, after, delayed, resumed);
        if (pending != IntrinsicsKt.getCOROUTINE_SUSPENDED() || finalized[0] != 1)
            throw new AssertionError("cleanup ran before resume");
        delayed.continuation.resumeWith("resumed");
        if (resumed.calls != 1 || finalized[0] != 2)
            throw new AssertionError("inline return after resume");
        System.out.println("OK");
    }
}
