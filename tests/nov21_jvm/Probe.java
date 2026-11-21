package novjvm.support;
public final class Probe {
    public static boolean staticHeld(String name) {
        try { return Thread.holdsLock(Class.forName(name)); }
        catch (ClassNotFoundException failure) { throw new AssertionError(failure); }
    }
}
