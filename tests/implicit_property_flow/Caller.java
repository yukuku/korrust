import model.Row;
import model.ConsumerKt;
public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.count(new Row(null, "x")) != 0 || ConsumerKt.count(new Row(3, "ab")) != 5 ||
            ConsumerKt.count(new Row(4, null)) != 4 || ConsumerKt.explicit(new Row(null, null)) != -1 ||
            ConsumerKt.explicit(new Row(9, null)) != 9) throw new AssertionError();
    }
}
