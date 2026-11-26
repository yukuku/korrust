import factory.*;
public class Caller {
    public static void main(String[] args) {
        if (!(ConsumerKt.items().get(0) instanceof First) || !(ConsumerKt.reverse().get(0) instanceof Second) ||
            !"factory.Base".equals(ConsumerKt.componentName())) throw new AssertionError();
    }
}
