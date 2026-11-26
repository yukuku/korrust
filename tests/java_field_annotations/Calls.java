package fieldcheck;
import fieldannotations.Fields;
import fieldconsumer.UsesKt;

public class Calls {
    public static void main(String[] args) {
        if (!UsesKt.text().equals("changed") || !Fields.text.equals("changed")) throw new AssertionError("static field");
        Fields fields = new Fields();
        if (!UsesKt.instance(fields).equals("item+") || !fields.item.equals("item+")) throw new AssertionError("instance field");
        if (!UsesKt.nonnull().equals("nonnull") || UsesKt.nullable() != null) throw new AssertionError("nullability");
        Fields.nullable = "present";
        if (!UsesKt.nullable().equals("present")) throw new AssertionError("nullable value");
        System.out.println("java-field-annotations-ok");
    }
}
