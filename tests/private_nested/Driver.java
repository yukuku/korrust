import privateprobe.Vault;
import java.lang.reflect.Modifier;
public class Driver {
    public static void main(String[] args) throws Exception {
        System.out.println(new Vault().read());
        System.out.println(new Vault().closure().invoke());
        System.out.println(new Vault.Sibling().read());
        System.out.println(Vault.Companion.read());
        System.out.println(privateprobe.ObjectOwner.INSTANCE.read());
        System.out.println(new privateprobe.Depth().read());
        System.out.println(new privateprobe.Depth().viaPrivate());
        for (String name : new String[]{"Entry", "Record", "Reader"}) {
            Class<?> cls = Class.forName("privateprobe.Vault$" + name);
            System.out.println(name + ":" + Modifier.toString(cls.getModifiers()));
        }
    }
}
