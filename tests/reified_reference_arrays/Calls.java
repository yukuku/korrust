package erasedcheck;
import erasedconsumer.UsesKt;
import erasedallocation.Box;

public class Calls {
    public static void main(String[] args) {
        Class<?>[] classes = UsesKt.classes();
        if (classes.getClass().getComponentType() != Class.class || classes.length != 2 || classes[0] != String.class || classes[1] != int.class) throw new AssertionError("class values");
        if (UsesKt.emptyClasses().getClass().getComponentType() != Class.class || UsesKt.emptyClasses().length != 0) throw new AssertionError("empty array");
        Object[] nullable = UsesKt.nullableClasses();
        if (nullable.getClass().getComponentType() != Class.class || nullable.length != 2 || nullable[0] != null) throw new AssertionError("nullable class array");
        nullable[0] = Integer.class;
        boolean failed = false;
        try { nullable[1] = "wrong"; } catch (ArrayStoreException expected) { failed = true; }
        if (!failed) throw new AssertionError("class array stores");
        Object[] boxes = UsesKt.boxes();
        if (boxes.getClass().getComponentType() != Box.class || boxes.length != 2 || boxes[0] != null) throw new AssertionError("box array");
        boxes[0] = new Box<String>("value");
        Object[] lists = UsesKt.lists();
        if (lists.getClass().getComponentType() != java.util.List.class || lists.length != 2 || lists[0] != null) throw new AssertionError("list array");
        lists[0] = new java.util.ArrayList<String>();
        Object[] generic = UsesKt.<String>genericClasses(3);
        if (generic.getClass().getComponentType() != Class.class || generic.length != 3) throw new AssertionError("erased nested parameter");
        System.out.println("reified-reference-arrays-ok");
    }
}
