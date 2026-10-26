import compatibility.annotations.Annotated;
import compatibility.annotations.RuntimeMark;
import compatibility.visibility.InternalObject;
import compatibility.visibility.InternalClass;
import compatibility.visibility.FilePrivateAKt;
import compatibility.visibility.FilePrivateBKt;
import compatibility.visibility.PrivateKt;
import compatibility.visibility.SameModuleKt;
import java.lang.reflect.AnnotatedElement;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;

public final class AnnotationsVisibilityCheck {
    private static void mark(AnnotatedElement element, String text, int count, boolean enabled) {
        RuntimeMark annotation = element.getAnnotation(RuntimeMark.class);
        if (annotation == null || !annotation.text().equals(text) || annotation.count() != count || annotation.enabled() != enabled) {
            throw new AssertionError("annotation " + text);
        }
    }

    public static void main(String[] args) throws Exception {
        mark(Annotated.class, "type", 1, true);
        mark(Annotated.class.getDeclaredField("value"), "field", 3, true);
        mark(Annotated.class.getMethod("getValue"), "getter", 4, false);
        mark(Annotated.class.getMethod("setValue", int.class), "setter", 5, true);
        mark(Annotated.class.getMethod("plus", int.class), "method", 6, false);
        Constructor<Annotated> constructor = Annotated.class.getConstructor(int.class);
        mark(constructor.getParameters()[0], "parameter", 2, false);
        mark(Annotated.class.getMethod("setValue", int.class).getParameters()[0], "setter-parameter", 7, false);
        Annotated value = constructor.newInstance(4);
        if (value.plus(3) != 7) throw new AssertionError("method");
        value.setValue(9);
        if (value.plus(1) != 10) throw new AssertionError("setter");
        if (SameModuleKt.sameModule() != 23 || PrivateKt.visible(2) != 20) throw new AssertionError("private/module");
        if (InternalObject.INSTANCE.decorate$visibility_fixture(3) != 10) throw new AssertionError("internal mangling");
        if (InternalObject.INSTANCE.sameFilePrivate(2) != 24) throw new AssertionError("same-file cross-owner private access");
        InternalClass internalClass = new InternalClass();
        if (internalClass.internalValue$visibility_fixture(4) != 23 || internalClass.publicValue(5) != 24) {
            throw new AssertionError("class internal mangling/public call");
        }
        if (FilePrivateAKt.privateA() != 6 || FilePrivateBKt.privateB() != 10) throw new AssertionError("file scopes");
        Method privateMethod = PrivateKt.class.getDeclaredMethod("privateOffset", int.class);
        if (!java.lang.reflect.Modifier.isPrivate(privateMethod.getModifiers())) throw new AssertionError("private JVM method");
        System.out.println("annotations-visibility-ok:13");
    }
}
