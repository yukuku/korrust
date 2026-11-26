import fixtures.*;
public class Caller {
    public static void main(String[] args) {
        Bean bean = new Bean();
        ConsumerKt.exercise(bean);
        if (bean.writes != 4) throw new AssertionError(bean.writes);
    }
}
