import markers.*;
public class Caller {
    public static void main(String[] args) {
        Class<?>[] entries = Marked.class.getAnnotation(Entries.class).classes();
        if (entries.length != 2 || entries[0] != Child.class || entries[1] != Base.class
                || Marked.class.getAnnotation(Flags.class).value()[0] != Flag.class)
            throw new AssertionError();
    }
}
