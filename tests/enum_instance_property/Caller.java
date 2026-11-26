import row.Column;
public class Caller {
    public static void main(String[] args) {
        if (!"name".equals(ConsumerKt.name()) || !"title".equals(ConsumerKt.title())
            || ConsumerKt.index(Column.ordinal) != 1) throw new AssertionError();
    }
}
