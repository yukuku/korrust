package fieldmeta;
import fieldmeta.dependency.*;
import java.util.Arrays;
public final class Calls {
    public static void main(String[] args) throws Exception {
        if (ModelsKt.javaConstant() != JavaMode.TYPE_USE) throw new AssertionError();
        if (ModelsKt.first() != Choice.FIRST || ModelsKt.pick(false) != Choice.SECOND) throw new AssertionError();
        if (ModelsKt.route(Choice.FIRST) != 1 || ModelsKt.route(Choice.SECOND) != 2) throw new AssertionError();
        Label first = Choice.class.getField("FIRST").getAnnotation(Label.class);
        if (!first.label().equals("one") || first.count() != 3 || !Arrays.equals(first.names(), new String[]{"x", "y"})) throw new AssertionError();
        Label second = Choice.class.getField("SECOND").getAnnotation(Label.class);
        if (!second.label().equals("two") || second.count() != 4 || second.names().length != 0) throw new AssertionError();
        System.out.println("OK");
    }
}
