import valueinner.Outer;
import valueinner.ProviderKt;

public class Check {
    public static void main(String[] args) {
        Outer first = new Outer(10);
        Outer second = new Outer(100);
        Outer.Reader reader = first.new Reader(3);
        Outer.Reader other = second.new Reader(5);
        if (reader.read() != 13 || other.read() != 105) throw new AssertionError("outer identity");
        if (reader.update(20) != 23 || first.getBase() != 20) throw new AssertionError("outer mutation");
        if (other.read() != 105) throw new AssertionError("independent outer");
        if (ProviderKt.count() != 2) throw new AssertionError("unrelated enum");
        if (!ProviderKt.identityChecks()) throw new AssertionError("nullable value identity against null");
        System.out.println("OK");
    }
}
