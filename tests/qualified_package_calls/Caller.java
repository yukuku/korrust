import sample.ConsumerKt;
import sample.Local;

public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.intValue() != 9 || !ConsumerKt.textValue().equals("ok!") ||
            ConsumerKt.shadow(new Local()) != 8) throw new AssertionError();
    }
}
