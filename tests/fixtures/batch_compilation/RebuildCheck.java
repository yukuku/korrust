public final class RebuildCheck {
    public static void main(String[] args) throws Exception {
        if (batch.changed.PrimaryKt.retained() != 11) throw new AssertionError("retained");
        for (String removed : new String[]{"batch.rebuild.PrimaryKt", "batch.rebuild.RemovedKt", "batch.rebuild.Tree$Child"}) {
            try { Class.forName(removed); throw new AssertionError("stale " + removed); }
            catch (ClassNotFoundException expected) { }
        }
        System.out.println("rebuild-ok:11");
    }
}
