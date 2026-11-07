package filecheck;

import filemarks.Visible;

public final class Check {
    public static void main(String[] args) throws Exception {
        if (fileuses.UsesKt.answer() != 42 || !fileuses.UsesKt.getLabel().equals("value")) throw new AssertionError();
        Visible annotation = fileuses.UsesKt.class.getAnnotation(Visible.class);
        if (annotation == null || !annotation.text().equals("hello") || annotation.number() != 7) throw new AssertionError();
        if (fileuses.UsesKt.class.getDeclaredAnnotations().length != 2) throw new AssertionError();
        for (Class<?> owner : new Class<?>[]{fileuses.Neighbor.class, fileuses.Standalone.class, fileuses.OtherKt.class, fileuses.SourceOnlyKt.class}) {
            if (owner.getAnnotation(Visible.class) != null) throw new AssertionError(owner);
        }
        try { Class.forName("fileuses.ClassFileKt"); throw new AssertionError(); }
        catch (ClassNotFoundException expected) {}
        if (!fileshadow.ShadowKt.class.getAnnotation(fileshadow.OptIn.class).text().equals("ordinary")) throw new AssertionError();
        System.out.println("OK:42:hello:7");
    }
}
