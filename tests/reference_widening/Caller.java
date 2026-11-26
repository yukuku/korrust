import consumers.Fields;
public class Caller {
    public static void main(String[] args) {
        Fields fields = new Fields(null);
        if (fields.viaJava() != null || fields.read() != null) throw new AssertionError();
        String next = new String("value");
        fields.assign(next);
        if (fields.viaJava() != next || fields.read() != next) throw new AssertionError();
    }
}
