import composition.ExtendedKt;
import composition.Source;
import kotlin.ResultKt;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;

public final class Calls {
    static class Done implements Continuation<String> {
        int calls;
        Object value;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object result) {
            calls++;
            ResultKt.throwOnFailure(result);
            value = result;
        }
    }

    static class Pause implements Function2<String, Continuation<? super String>, Object> {
        Continuation<? super String> saved;
        public Object invoke(String text, Continuation<? super String> c) {
            if (!"verse 一".equals(text)) throw new AssertionError(text);
            saved = c;
            return IntrinsicsKt.getCOROUTINE_SUSPENDED();
        }
    }

    public static void main(String[] args) {
        Source source = ExtendedKt.reader("一");
        Done done = new Done();
        Pause pause = new Pause();
        Object initial = ExtendedKt.reference().invoke(source, pause, done);
        if (initial != IntrinsicsKt.getCOROUTINE_SUSPENDED() || done.calls != 0)
            throw new AssertionError("Returned before suspension");
        pause.saved.resumeWith("1 一");
        if (done.calls != 1 || !"1 一".equals(done.value))
            throw new AssertionError("Incorrect resumption");
        System.out.println("OK");
    }
}
