import anonymousmembers.Counted;
import anonymousmembers.MembersKt;
public final class Check {
    public static void main(String[] args) {
        Counted first = MembersKt.stored(4);
        Counted second = MembersKt.stored(9);
        if (first.add(3) != 7 || second.getCount() != 9) throw new AssertionError("fresh state");
        first.setLabel("changed");
        if (!first.getLabel().equals("changed")) throw new AssertionError("setter");
        Counted computed = MembersKt.computed(5);
        computed.setLabel("value");
        if (computed.add(2) != 7 || !computed.getLabel().equals("value7")) throw new AssertionError("accessors");
        if (MembersKt.localMembers(4) != 1414) throw new AssertionError("local members");
        if (MembersKt.ordered().getCount() != 2 || !MembersKt.getEffects().equals("ab")) throw new AssertionError("initialization order");
        MembersKt.setEffects("");
        if (MembersKt.inherited(6).add(2) != 8 || !MembersKt.getEffects().equals("spi")) throw new AssertionError("superclass initialization");
        if (new anonymousmembers.Outer(10).make().add(2) != 16) throw new AssertionError("own and outer receivers");
        System.out.println("OK");
    }
}
