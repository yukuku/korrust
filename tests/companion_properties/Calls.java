package companionstatecheck;
import companionstate.Store;
import companionstate.Shadow;
import companionstate.LabelUser;
public class Calls {
    public static void main(String[] args) {
        Store store = new Store(3);
        if (store.read() != 9 || !Store.raw.equals("init")) throw new AssertionError("constructor");
        if (store.assign(4) != 12 || !Store.raw.equals("assign")) throw new AssertionError("custom setter");
        if (store.delayed(5).invoke() != 15 || !Store.raw.equals("lambda")) throw new AssertionError("lambda");
        if (store.local(7) != 8 || Store.Companion.getCustom() != 10) throw new AssertionError("local shadow");
        Shadow shadow = new Shadow();
        if (!shadow.change("changed").equals("changed") || !shadow.getCount().equals("changed") || Shadow.Companion.getCount() != 70) throw new AssertionError("instance shadow");
        if (!new LabelUser().first().equals("interface-label")) throw new AssertionError("interface companion binding");
        System.out.println("companion-properties-ok");
    }
}
