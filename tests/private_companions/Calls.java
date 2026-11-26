package companioncalls;
import companionhidden.Vault;
public class Calls {
    public static void main(String[] args) throws Exception {
        Vault vault = new Vault();
        if (!vault.read().equals("private-companion-ok")) throw new AssertionError();
        if (!vault.qualified().equals(vault.read()) || !vault.explicit().equals(vault.read())) throw new AssertionError();
        if (!vault.called().equals(vault.read()) || !vault.property().equals(vault.read())) throw new AssertionError("private members");
        if (!vault.shadow("local-shadow").equals("local-shadow")) throw new AssertionError("lexical shadowing");
        if (!Vault.TOKEN.equals(vault.read())) throw new AssertionError("JVM public const");
        java.lang.reflect.Field singleton = Vault.class.getDeclaredField("Companion");
        if (singleton.getModifiers() != 0x1a) throw new AssertionError("singleton visibility");
        Class<?> companion = Class.forName("companionhidden.Vault$Companion");
        if (companion.getModifiers() != 0x1a) throw new AssertionError("InnerClasses visibility");
        System.out.println("private-companion-ok");
    }
}
