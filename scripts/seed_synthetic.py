#!/usr/bin/env python3
"""Seed a scratch Camunda 7 engine with synthetic data only, for screenshots and the demo GIF.

Deploys:
  * crates/bpmn/tests/fixtures/order.bpmn (`order_process`) with instances ORD-1001…, parked at
    user tasks, timers and message events (the first service task has no delegate, so instances
    start with startBeforeActivity);
  * a generated large process (`fleet_maintenance`, 36 stages in 6 lanes, ~700 elements) that
    shows how the viewport copes with big diagrams;
  * a tiny `leave_request` process so lists have more than one row.

Usage: scripts/seed_synthetic.py [rest_base]   (default http://localhost:8089/engine-rest)
       scripts/seed_synthetic.py --write-big out.bpmn   (only write the generated diagram)

Never point this at an engine holding real data: it is meant for an empty scratch engine."""
import json, os, sys, urllib.request, uuid

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------- big synthetic diagram

LANES = ["Dispatch", "Workshop", "Parts", "Quality", "Finance", "Customer care"]
STAGES = [
    "Intake", "Triage", "Inspection", "Diagnostics", "Estimate", "Approval",
    "Parts check", "Ordering", "Receiving", "Kitting", "Staging", "Allocation",
    "Teardown", "Repair", "Rebuild", "Calibration", "Road test", "Rework",
    "Inspection II", "Certification", "Paperwork", "Sign-off", "Release", "Handover",
    "Invoicing", "Payment", "Reconciliation", "Warranty", "Claims", "Audit",
    "Follow-up", "Survey", "Feedback", "Loyalty", "Archive", "Close-out",
]
COLS = 6
COL_W, LANE_H = 1000, 420
BOX_W, BOX_H = 820, 300
POOL_LABEL = 30


def big_bpmn():
    ns = ('xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" '
          'xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" '
          'xmlns:dc="http://www.omg.org/spec/DD/20100524/DC" '
          'xmlns:di="http://www.omg.org/spec/DD/20100524/DI" '
          'xmlns:camunda="http://camunda.org/schema/1.0/bpmn"')
    proc, shapes, edges = [], [], []
    lane_refs = {i: [] for i in range(len(LANES))}

    def shape(eid, x, y, w, h, expanded=False):
        exp = ' isExpanded="true"' if expanded else ""
        shapes.append(f'<bpmndi:BPMNShape id="{eid}_di" bpmnElement="{eid}"{exp}>'
                      f'<dc:Bounds x="{x}" y="{y}" width="{w}" height="{h}" /></bpmndi:BPMNShape>')

    def edge(eid, pts):
        wp = "".join(f'<di:waypoint x="{x}" y="{y}" />' for x, y in pts)
        edges.append(f'<bpmndi:BPMNEdge id="{eid}_di" bpmnElement="{eid}">{wp}</bpmndi:BPMNEdge>')

    def flow(out, eid, src, dst, pts, cond=None):
        body = f'<bpmn:conditionExpression xsi:type="bpmn:tFormalExpression">{cond}</bpmn:conditionExpression>' if cond else ""
        out.append(f'<bpmn:sequenceFlow id="{eid}" sourceRef="{src}" targetRef="{dst}">{body}</bpmn:sequenceFlow>')
        edge(eid, pts)

    def box(k):
        lane, col = divmod(k, COLS)
        if lane % 2:  # snake: odd lanes run right to left
            col = COLS - 1 - col
        x = POOL_LABEL + 30 + 150 + col * COL_W
        y = lane * LANE_H + (LANE_H - BOX_H) // 2
        return lane, x, y

    width = POOL_LABEL + 30 + 150 + COLS * COL_W + 100
    height = len(LANES) * LANE_H

    # process start / end
    _, x0, y0 = box(0)
    proc.append('<bpmn:startEvent id="Start" name="Vehicle booked in" />')
    shape("Start", x0 - 120, y0 + BOX_H // 2 - 18, 36, 36)
    lane_refs[0].append("Start")

    for k, name in enumerate(STAGES):
        lane, x, y = box(k)
        sid = f"S{k:02d}"
        lane_refs[lane].append(sid)
        inner = []
        p = sid + "_"
        el = {  # id: (xml, x, y, w, h)
            "start": (f'<bpmn:startEvent id="{p}start" />', 30, 132, 36, 36),
            "collect": (f'<bpmn:userTask id="{p}collect" name="{name}: collect details" camunda:candidateGroups="{LANES[lane].lower().replace(" ", "-")}" />', 100, 110, 100, 80),
            "gw": (f'<bpmn:exclusiveGateway id="{p}gw" name="Priority?" default="{p}f_std" />', 250, 125, 50, 50),
            "std": (f'<bpmn:userTask id="{p}std" name="Standard review" />', 340, 40, 100, 80),
            "prio": (f'<bpmn:serviceTask id="{p}prio" name="Priority review" camunda:expression="${{true}}" />', 340, 180, 100, 80),
            "join": (f'<bpmn:exclusiveGateway id="{p}join" />', 490, 125, 50, 50),
            "sign": (f'<bpmn:userTask id="{p}sign" name="{name}: sign off" camunda:assignee="${{owner}}" />', 580, 110, 100, 80),
            "end": (f'<bpmn:endEvent id="{p}end" name="Done" />', 740, 132, 36, 36),
            "late": (f'<bpmn:endEvent id="{p}late" name="Escalated" />', 740, 232, 36, 36),
        }
        for key, (xml, ex, ey, ew, eh) in el.items():
            inner.append(xml)
            shape(p + key, x + ex, y + ey, ew, eh)
        inner.append(f'<bpmn:boundaryEvent id="{p}timer" attachedToRef="{p}sign">'
                     f'<bpmn:timerEventDefinition><bpmn:timeDuration xsi:type="bpmn:tFormalExpression">P{1 + k % 5}D</bpmn:timeDuration>'
                     f'</bpmn:timerEventDefinition></bpmn:boundaryEvent>')
        shape(p + "timer", x + 612, y + 172, 36, 36)

        def c(key, side):
            _, ex, ey, ew, eh = el[key]
            return {"r": (x + ex + ew, y + ey + eh // 2), "l": (x + ex, y + ey + eh // 2),
                    "t": (x + ex + ew // 2, y + ey), "b": (x + ex + ew // 2, y + ey + eh)}[side]
        flow(inner, p + "f1", p + "start", p + "collect", [c("start", "r"), c("collect", "l")])
        flow(inner, p + "f2", p + "collect", p + "gw", [c("collect", "r"), c("gw", "l")])
        flow(inner, p + "f_std", p + "gw", p + "std", [c("gw", "t"), (c("gw", "t")[0], c("std", "l")[1]), c("std", "l")])
        flow(inner, p + "f_prio", p + "gw", p + "prio", [c("gw", "b"), (c("gw", "b")[0], c("prio", "l")[1]), c("prio", "l")],
             cond="${priority == 'high'}")
        flow(inner, p + "f3", p + "std", p + "join", [c("std", "r"), (c("join", "t")[0], c("std", "r")[1]), c("join", "t")])
        flow(inner, p + "f4", p + "prio", p + "join", [c("prio", "r"), (c("join", "b")[0], c("prio", "r")[1]), c("join", "b")])
        flow(inner, p + "f5", p + "join", p + "sign", [c("join", "r"), c("sign", "l")])
        flow(inner, p + "f6", p + "sign", p + "end", [c("sign", "r"), c("end", "l")])
        tb = (x + 630, y + 208)
        flow(inner, p + "f7", p + "timer", p + "late", [tb, (tb[0], c("late", "l")[1]), c("late", "l")])
        proc.append(f'<bpmn:subProcess id="{sid}" name="{k + 1}. {name}">' + "".join(inner) + "</bpmn:subProcess>")
        shape(sid, x, y, BOX_W, BOX_H, expanded=True)

        # connect from the previous stage (or the start event)
        if k == 0:
            flow(proc, "F00", "Start", sid, [(x0 - 84, y0 + BOX_H // 2), (x, y + BOX_H // 2)])
        else:
            plane, px, py = box(k - 1)
            if plane != lane:  # down to the next lane
                pts = [(px + BOX_W // 2, py + BOX_H), (x + BOX_W // 2, y)]
            elif px < x:
                pts = [(px + BOX_W, py + BOX_H // 2), (x, y + BOX_H // 2)]
            else:
                pts = [(px, py + BOX_H // 2), (x + BOX_W, y + BOX_H // 2)]
            flow(proc, f"F{k:02d}", f"S{k - 1:02d}", sid, pts)

    lane, xl, yl = box(len(STAGES) - 1)
    ex = xl - 120 if lane % 2 else xl + BOX_W + 80
    proc.append('<bpmn:endEvent id="End" name="Vehicle returned" />')
    shape("End", ex, yl + BOX_H // 2 - 18, 36, 36)
    lane_refs[lane].append("End")
    pts = [(xl, yl + BOX_H // 2), (ex + 36, yl + BOX_H // 2)] if lane % 2 else [(xl + BOX_W, yl + BOX_H // 2), (ex, yl + BOX_H // 2)]
    flow(proc, "F_end", f"S{len(STAGES) - 1:02d}", "End", pts)

    lanes_xml = []
    for i, name in enumerate(LANES):
        refs = "".join(f"<bpmn:flowNodeRef>{r}</bpmn:flowNodeRef>" for r in lane_refs[i])
        lanes_xml.append(f'<bpmn:lane id="Lane_{i}" name="{name}">{refs}</bpmn:lane>')
        shape(f"Lane_{i}", POOL_LABEL, i * LANE_H, width - POOL_LABEL, LANE_H)
    shapes.insert(0, f'<bpmndi:BPMNShape id="Pool_di" bpmnElement="Pool" isHorizontal="true">'
                     f'<dc:Bounds x="0" y="0" width="{width}" height="{height}" /></bpmndi:BPMNShape>')
    return (f'<?xml version="1.0" encoding="UTF-8"?>\n'
            f'<bpmn:definitions {ns} xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" id="Definitions_fleet" '
            f'targetNamespace="http://flightdeck.test" exporter="flightdeck seed_synthetic.py" exporterVersion="1">'
            f'<bpmn:collaboration id="Collab"><bpmn:participant id="Pool" name="Fleet maintenance" processRef="fleet_maintenance" /></bpmn:collaboration>'
            f'<bpmn:process id="fleet_maintenance" name="Fleet maintenance" isExecutable="true" camunda:historyTimeToLive="30">'
            f'<bpmn:documentation>Synthetic large process generated by scripts/seed_synthetic.py for viewport demos.</bpmn:documentation>'
            f'<bpmn:laneSet id="LaneSet_1">{"".join(lanes_xml)}</bpmn:laneSet>{"".join(proc)}</bpmn:process>'
            f'<bpmndi:BPMNDiagram id="Diagram_1"><bpmndi:BPMNPlane id="Plane_1" bpmnElement="Collab">'
            f'{"".join(shapes)}{"".join(edges)}</bpmndi:BPMNPlane></bpmndi:BPMNDiagram></bpmn:definitions>\n')


LEAVE = """<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC" xmlns:di="http://www.omg.org/spec/DD/20100524/DI" xmlns:camunda="http://camunda.org/schema/1.0/bpmn" id="Definitions_leave" targetNamespace="http://flightdeck.test">
  <bpmn:process id="leave_request" name="Leave request" isExecutable="true" camunda:historyTimeToLive="30">
    <bpmn:startEvent id="Start"><bpmn:outgoing>F1</bpmn:outgoing></bpmn:startEvent>
    <bpmn:userTask id="Approve" name="Approve leave" camunda:candidateGroups="managers"><bpmn:incoming>F1</bpmn:incoming><bpmn:outgoing>F2</bpmn:outgoing></bpmn:userTask>
    <bpmn:endEvent id="End" name="Decided"><bpmn:incoming>F2</bpmn:incoming></bpmn:endEvent>
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Approve" />
    <bpmn:sequenceFlow id="F2" sourceRef="Approve" targetRef="End" />
  </bpmn:process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane id="P" bpmnElement="leave_request">
    <bpmndi:BPMNShape id="Start_di" bpmnElement="Start"><dc:Bounds x="100" y="100" width="36" height="36" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="Approve_di" bpmnElement="Approve"><dc:Bounds x="190" y="78" width="100" height="80" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="End_di" bpmnElement="End"><dc:Bounds x="350" y="100" width="36" height="36" /></bpmndi:BPMNShape>
    <bpmndi:BPMNEdge id="F1_di" bpmnElement="F1"><di:waypoint x="136" y="118" /><di:waypoint x="190" y="118" /></bpmndi:BPMNEdge>
    <bpmndi:BPMNEdge id="F2_di" bpmnElement="F2"><di:waypoint x="290" y="118" /><di:waypoint x="350" y="118" /></bpmndi:BPMNEdge>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</bpmn:definitions>
"""

# ---------------------------------------------------------------- REST helpers


def rest(base, method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(base + path, data, {"content-type": "application/json"}, method=method)
    raw = urllib.request.urlopen(req, timeout=60).read()
    return json.loads(raw) if raw else None


def deploy(base, name, filename, xml):
    b = "----flightdeck" + uuid.uuid4().hex
    parts = [f'--{b}\r\nContent-Disposition: form-data; name="deployment-name"\r\n\r\n{name}\r\n',
             f'--{b}\r\nContent-Disposition: form-data; name="deploy-changed-only"\r\n\r\ntrue\r\n',
             f'--{b}\r\nContent-Disposition: form-data; name="data"; filename="{filename}"\r\n'
             f'Content-Type: application/octet-stream\r\n\r\n']
    body = "".join(parts).encode() + xml.encode() + f"\r\n--{b}--\r\n".encode()
    req = urllib.request.Request(base + "/deployment/create", body,
                                 {"content-type": f"multipart/form-data; boundary={b}"}, method="POST")
    urllib.request.urlopen(req, timeout=60).read()


def var(v):
    t = "Boolean" if isinstance(v, bool) else "Long" if isinstance(v, int) else "String"
    return {"value": v, "type": t}


def start(base, key, bk, before, variables):
    rest(base, "POST", f"/process-definition/key/{key}/start", {
        "businessKey": bk,
        "variables": {k: var(v) for k, v in variables.items()},
        "startInstructions": [{"type": "startBeforeActivity", "activityId": before}],
    })


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--write-big":
        open(sys.argv[2], "w").write(big_bpmn())
        return
    base = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:8089/engine-rest"
    if rest(base, "GET", "/process-definition/count")["count"]:
        sys.exit(f"{base} already has deployments; seed an empty scratch engine only")
    order = open(os.path.join(ROOT, "crates/bpmn/tests/fixtures/order.bpmn")).read()
    deploy(base, "order", "order.bpmn", order)
    deploy(base, "fleet", "fleet_maintenance.bpmn", big_bpmn())
    deploy(base, "leave", "leave_request.bpmn", LEAVE)
    parks = ["Task_Approve", "Event_Wait", "Task_Ship"]
    for i in range(1, 7):
        for j, at in enumerate(parks):
            amount = 1500 + 2000 * j + 500 * i
            start(base, "order_process", f"ORD-{i}00{j + 1}", at,
                  {"amount": amount, "approver": "demo", "customer": f"Big Corp {j + 1}", "seen": False})
    start(base, "fleet_maintenance", "VAN-0042", "S27_sign", {"owner": "demo", "priority": "normal"})
    start(base, "leave_request", "LR-7", "Approve", {"days": 3})
    print("seeded", rest(base, "GET", "/process-instance/count")["count"], "instances")


if __name__ == "__main__":
    main()
