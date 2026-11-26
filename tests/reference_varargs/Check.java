import referencevarargs.*;
public final class Check {
    public static void main(String[] args) {
        if (!ValuesKt.join("a", "b").equals("ab") || !ValuesKt.join().equals("")) throw new AssertionError("scalar calls");
        if (ValuesKt.nullable("a", null, "bb") != 3) throw new AssertionError("nullable elements");
        if (ValuesKt.objects(new Item("abc"), new Item("x")) != 5) throw new AssertionError("objects");
        if (!ValuesKt.capture("captured").invoke().equals("captured")) throw new AssertionError("capture");
        String[] input = {"b", "c"};
        if (ValuesKt.expose(input) != input) throw new AssertionError("callee identity");
        if (!ValuesKt.generic("first", "last").equals("first")) throw new AssertionError("generic");
        if (ValuesKt.generic((String) null) != null) throw new AssertionError("nullable generic");
        if (ValuesKt.projection(input) != 1 || !ValuesKt.forwarded(input).equals("xbcz")) throw new AssertionError("projected calls");
        if (!ValuesKt.memberGet("get").equals("get") || !ValuesKt.mutableSet(new String[]{"x"}).equals("set")) throw new AssertionError("array members");
        if (!ValuesKt.isolation(input).equals("java:b") || !input[0].equals("b")) throw new AssertionError("spread isolation");
        if (ValuesKt.namedShares(input)) throw new AssertionError("named spread isolation");
        if (!ValuesKt.ordered(input).equals("abcd") || !ValuesKt.getEffects().equals("123")) throw new AssertionError("evaluation order");
        Bag bag = new Holder(10);
        if (bag.count("a", "b") != 12 || !bag.first("first").equals("first")) throw new AssertionError("interface dispatch");
        if (Named.INSTANCE.count(new Item("a"), new Item("b")) != 2) throw new AssertionError("object member");
        if (!new SourceGeneric().first("head").equals("head") || new Sink<String>().count("x", "y") != 2) throw new AssertionError("generic member");
        try { ValuesKt.join((String[]) null); throw new AssertionError("null array accepted"); } catch (NullPointerException expected) { }
        System.out.println("OK");
    }
}
