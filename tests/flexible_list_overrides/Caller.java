import java.util.*;
public class Caller {
    public static void main(String[] args) {
        Receiver mutable = new MutableReceiver(), reader = ConsumerKt.readOnly();
        if (mutable.accept(null) != -1 || mutable.accept(Arrays.asList("a","b")) != 2 ||
                reader.accept(Arrays.asList("a", null, "c")) != 3) throw new AssertionError();
    }
}
