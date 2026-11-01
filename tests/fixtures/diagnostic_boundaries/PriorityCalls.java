public class PriorityCalls {
    public static void main(String[] args) throws Exception {
        if (!PriorityKt.priorityProbe().equals("high")) throw new AssertionError("wrong import group");
        if (!localcase.LocalPriorityKt.localPriorityProbe().equals("high")) throw new AssertionError("lower same-package group");
        System.out.println("OK");
    }
}
