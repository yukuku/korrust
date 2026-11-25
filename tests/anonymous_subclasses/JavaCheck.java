import anonymousjava.*;
public class JavaCheck {
    public static void main(String[] args) throws Exception {
        JavaBase value = JavaKt.javaBase(4);
        if (value.initial() != 11 || value.read() != 9) throw new AssertionError();
        ((Runnable) value).run();
        if (value.compute(3) != 14 || value.read() != 11) throw new AssertionError();
        if (JavaKt.stream(37).read() != 37) throw new AssertionError();
        System.out.println("OK");
    }
}
