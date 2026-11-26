import chars.ConsumerKt;
import chars.Steps;

public class Caller {
    public static void main(String[] args) {
        for (char ch : new char[]{0, 'A', 0x7fff, 0xd800, 0xffff}) {
            for (int n : new int[]{Integer.MIN_VALUE, -1, 0, 1, 65536, Integer.MAX_VALUE}) {
                if (ConsumerKt.forward(ch, n) != (char) (ch + n)) throw new AssertionError();
                if (ConsumerKt.backward(ch, n) != (char) (ch - n)) throw new AssertionError();
            }
            if (ConsumerKt.difference(ch, (char) 0xffff) != ch - 65535) throw new AssertionError();
            if (ConsumerKt.difference((char) 0xffff, ch) != 65535 - ch) throw new AssertionError();
        }
        if (ConsumerKt.observed() != 0 || Steps.trace != 12) throw new AssertionError();
    }
}
