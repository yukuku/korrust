import consumers.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        String[] values = {"one", null};
        if (ConsumerKt.sizes(values, new String[1][0], new int[3][0]) != 8) throw new AssertionError();
        try {
            ConsumerKt.write(values);
            throw new AssertionError();
        } catch (ArrayStoreException expected) {
            if (!"one".equals(values[0])) throw new AssertionError();
        }
    }
}
