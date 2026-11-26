import contextlib.*;
import kotlin.coroutines.CoroutineContext;
public class Caller {
    public static void main(String[] args) {
        Token token = new Token(), other = new Token();
        Flag flag = new Flag();
        for (CoroutineContext result : new CoroutineContext[] {
                ConsumerKt.combine(token, flag), ConsumerKt.named(token, flag)}) {
            if (result.get(Token.Key) != token || result.get(Flag.Key) != flag)
                throw new AssertionError();
        }
        if (ConsumerKt.specific(token, other) != other) throw new AssertionError();
    }
}
