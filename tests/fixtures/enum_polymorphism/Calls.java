package enums.polymorphism;

import java.lang.reflect.Modifier;
import java.util.Arrays;

public class Calls {
    private static void expect(Object expected, Object actual) {
        if (!expected.equals(actual)) throw new AssertionError(expected + " != " + actual);
    }
    public static void main(String[] args) {
        expect("L27", Task.LEFT.run(6));
        expect("R14", Task.RIGHT.run(6));
        expect("E9", Task.EMPTY.run(6));
        expect("sflid", Trace.INSTANCE.getLog());
        expect("LEFT:4!", Task.LEFT.getCaption());
        expect("companion", Task.Companion.getMarker());
        expect(Task.LEFT, Task.Companion.first());
        expect("RIGHT:3", Task.RIGHT.getCaption());
        Action action = Task.LEFT;
        expect("L23", action.run(2));
        expect("LEFT:4!", action.getCaption());
        expect(8.5, AbstractAction.A.score());
        expect(12.5, AbstractAction.B.score());
        expect("a2", PerEntry.A.run(2));
        expect("B", PerEntry.B.getCaption());
        expect(7, DefaultOnly.A.getValue());
        expect(8, DefaultOnly.A.getFollowing());
        expect(12, DefaultOnly.B.getFollowing());
        expect(7, DefaultOnly.C.getValue());
        expect(20, DefaultOnly.C.getFollowing());
        expect("sflidvv", Trace.INSTANCE.getLog());
        expect(Task.class, Task.LEFT.getDeclaringClass());
        expect("enums.polymorphism.Task$LEFT", Task.LEFT.getClass().getName());
        expect(Task.class, Task.EMPTY.getClass());
        expect(false, Modifier.isPublic(Task.LEFT.getClass().getModifiers()));
        expect(true, Modifier.isFinal(Task.LEFT.getClass().getModifiers()));
        expect(true, Modifier.isAbstract(AbstractAction.class.getModifiers()));
        expect(true, Modifier.isAbstract(PerEntry.class.getModifiers()));
        expect(false, Modifier.isFinal(Task.class.getModifiers()));
        expect(true, Modifier.isFinal(DefaultOnly.class.getModifiers()));
        Task[] values = Task.values();
        values[0] = Task.RIGHT;
        expect(Task.LEFT, Task.values()[0]);
        expect(Task.LEFT, Task.valueOf("LEFT"));
        expect(3, Task.getEntries().size());
        expect(Arrays.asList(Task.LEFT, Task.RIGHT, Task.EMPTY), Task.getEntries());
        System.out.println("OK");
    }
}
