import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "numeric_literals": check(LiteralsKt.literals() == 4294935864L); check(LiteralsKt.contextualLong() == 42L); break;
            case "mixed_declarations": check(MixedKt.before() == 9); check(MixedKt.after() == 3); break;
            case "uniform_bodies": check(BodiesKt.bodies() == 16); check(new BodyClass(3).getDoubled() == 6); check(new BodyClass(3).inferred() == 4); check(new BodyClass(3).weighted("abc", 1.5f) == 4.5f); check(BodyObject.INSTANCE.weighted("abc", 1.5f) == 4.5f); PrivateCapture capture = new PrivateCapture(7); check(capture.calc() == 8); check(capture.invokePrivate() == 8); break;
            case "singleton_state": check(StateKt.state() == 35); events("23"); check(StateKt.state() == 47); events("23"); break;
            case "init_blocks": check(InitializersKt.initializers() == 4); events("1234"); break;
            case "nested_declarations": check(NestedKt.nested() == 10); check(new Outer.Nested(6).getValue() == 6); check(Outer.Holder.INSTANCE.getValue() == 8); break;
            case "local_functions": check(LocalsKt.localFunctions(1) == 28); check(LocalsKt.localFunctions(2) == 52); break;
            case "operator_contains": Bag<String> bag = new Bag<>("yes"); check(MembershipKt.membership(bag,"yes")); check(MembershipKt.absent(bag,"no")); check(MembershipKt.concreteMembership(7)); check(!MembershipKt.concreteMembership(8)); events("11"); Trace.clear(); check(MembershipKt.containsOrder()); events("213"); break;
            case "generic_arrays": check(Arrays.equals(ArraysKt.makeArray(),new String[]{"v0","v1","v2"})); events("012"); check(ArraysKt.arrayTotal(new String[]{"ab",null,"c"}) == 3); check(ArraysKt.arrayTotal(new String[]{}) == 0); check(ArraysKt.integerArray() == 11); ArrayItem[] items = ArraysKt.objectArray(); check(items.getClass().getComponentType() == ArrayItem.class); check(items[0].getValue() == 4 && items[1].getValue() == 5); break;
            case "receiver_lambdas": check(ReceiversKt.receiverLambda().equals("a23")); StringBuilder receiver = new StringBuilder(); ReceiversKt.sourceReceiver(receiver, (builder, amount) -> { builder.append(amount); return kotlin.Unit.INSTANCE; }); check(receiver.toString().equals("4")); break;
            case "labeled_control": check(LabelsKt.labeledLoops() == 6); check(LabelsKt.labeledBreak() == 1); check(LabelsKt.labeledReturn() == 9); events("1112"); break;
            case "try_expressions": check(TryFlowKt.tryValue(4) == 5); check(TryFlowKt.tryValue(0) == 7); check(TryFlowKt.tryValue(-1) == 8); events("111"); Trace.clear(); check(TryFlowKt.finallyReturn() == 2); events("23"); check(TryFlowKt.finallyOverride() == 9); Trace.clear(); try { TryFlowKt.finallyThrow(); throw new AssertionError("missing throw"); } catch (IllegalStateException e) { check(e.getMessage().equals("original")); } events("4"); Trace.clear(); check(TryFlowKt.finallyBypass() == 8); events(""); Trace.clear(); check(TryFlowKt.nestedCatchFinally() == 5); events("3"); Trace.clear(); RuntimeException original = new RuntimeException("identity"); try { TryFlowKt.rethrowSame(original); throw new AssertionError("missing rethrow"); } catch(RuntimeException caught) { check(caught == original); } events("8"); check(TryFlowKt.tryReference(false).compute() == 5); check(TryFlowKt.tryReference(true).compute() == 9); check(TryFlowKt.mutableReference(false).compute() == 5); check(TryFlowKt.mutableReference(true).compute() == 9); break;
            case "operator_index": Slots<String> slots = new Slots<>("old"); check(OperatorsKt.readSlot(slots,0).equals("old")); events("2"); Trace.clear(); OperatorsKt.writeSlot(slots,"new"); events("03"); check(slots.getValue().equals("new")); check(OperatorsKt.concreteSlots() == 10); try { OperatorsKt.readSlot(slots,1); throw new AssertionError("missing bounds"); } catch (IndexOutOfBoundsException e) { check(true); } Trace.clear(); check(OperatorsKt.multiSlot(slots).equals("multi")); events("1254"); break;
            case "destructuring": Components components = new Components(); check(DestructureKt.destructure(components) == 7); events("12"); Trace.clear(); check(DestructureKt.destructureSkip(components).equals("abc")); events("2"); Trace.clear(); check(DestructureKt.destructureLoop(java.util.Arrays.asList(components, components)) == 14); events("1212"); Trace.clear(); check(DestructureKt.destructureLambda(components) == 7); events("12"); break;
            case "when_subject_variable": check(SubjectKt.subjectVariable(1) == 11); events("1"); Trace.clear(); check(SubjectKt.subjectVariable(3) == 23); events("3"); break;
            case "custom_accessors": check(AccessorsKt.accessorValue() == 13); events("21"); break;
            case "delegated_properties": check(DelegatesKt.delegatedValue() == 8); events("321"); Trace.clear(); DelegatedBox d1 = new DelegatedBox(); DelegatedBox d2 = new DelegatedBox(); d1.setScore(7); check(d1.getScore() == 7); check(d2.getScore() == 3); check(d1.getScore() == 7); events("332111"); break;
            case "lateinit_properties": check(LateinitKt.lateValue() == 5); try { new LateBox().getText(); throw new AssertionError("missing lateinit"); } catch (kotlin.UninitializedPropertyAccessException e) { check(e.getMessage().contains("text")); } LateBox late = new LateBox(); late.setText("ok"); check(late.getText().equals("ok")); break;
            case "inheritance_super": check(InheritanceKt.inherited(new Child(4)) == 7); check(InheritanceKt.inherited(new Parent(4)) == 5); check(InheritanceKt.abstractInherited(new ConcreteChild()) == 7); declarations.support.JavaBase javaBase = new JavaChild(4); check(javaBase.compute() == 12); check(((JavaChild) javaBase).protectedCapture() == 5); break;
            case "secondary_constructors": check(SecondaryKt.secondary() == 8); events("12123"); check(new SecondaryCollision(5).getStored() == 12); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}
