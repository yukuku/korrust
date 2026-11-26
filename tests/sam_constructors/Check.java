import samconstructors.UseKt;
public final class Check {
    public static void main(String[] args) {
        if (UseKt.imported(4).twice(2) != 12) throw new AssertionError("imported and default dispatch");
        if (UseKt.samePackage(4).run(3) != 12) throw new AssertionError("same package");
        if (UseKt.qualified(9).run(2) != 7) throw new AssertionError("qualified");
        if (UseKt.javaNested(4).apply(3) != 7) throw new AssertionError("Java nested");
        UseKt.javaDefault().run();
        if (UseKt.getEffects() != 1) throw new AssertionError("Java interface");
        if (!UseKt.generic().apply("value").equals("value!")) throw new AssertionError("generic bridge");
        if (UseKt.sourceDefined().apply(7) != 12) throw new AssertionError("source defined");
        var one = UseKt.capture();
        var two = UseKt.capture();
        if (one.run(2) != 3 || one.run(4) != 7 || two.run(1) != 2) throw new AssertionError("captured cells");
        System.out.println("OK");
    }
}
