import ranked.*;
import java.io.ByteArrayInputStream;

public class Caller {
    private static class Bytes extends ByteArrayInputStream {
        int closed;
        Bytes() { super(new byte[]{42}); }
        @Override public void close() { ++closed; }
    }
    public static void main(String[] args) {
        Child c = new Child();
        if (!ConsumerKt.chooseChild(c).equals("child:value")) throw new AssertionError();
        if (!ConsumerKt.chooseParent(c).equals("parent:value")) throw new AssertionError();
        Bytes b = new Bytes();
        if (ConsumerKt.read(b) != 42 || b.closed != 1) throw new AssertionError();
    }
}
