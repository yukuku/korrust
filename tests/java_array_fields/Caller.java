import fields.ConsumerKt;
import fields.Fields;

public class Caller {
    public static void main(String[] args) {
        Fields f = new Fields();
        if (ConsumerKt.word(f) != null || ConsumerKt.count(f) != 2) throw new AssertionError();
        if (ConsumerKt.matrix(f) != 2 || ConsumerKt.peer(f) != f) throw new AssertionError();
        if (!ConsumerKt.nested().equals("a")) throw new AssertionError();
        String[] values = {"new"};
        ConsumerKt.replace(f, values);
        if (f.words != values) throw new AssertionError();
    }
}
