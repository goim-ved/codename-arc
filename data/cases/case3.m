function mpc = case3
% MATPOWER case format for case3

mpc.version = '2';
mpc.baseMVA = 100.0;

% bus data
% bus_i type Pd Qd Gs Bs area Vm Va baseKV zone Vmax Vmin
mpc.bus = [
	1	3	0.0000	0.0000	0.0000	0.0000	1	1.0000	0.0000	138.0	1	2.0000	0.0000;
	2	1	40.0000	20.0000	0.0000	0.0000	1	1.0000	0.0000	138.0	1	2.0000	0.0000;
	3	2	0.0000	0.0000	0.0000	0.0000	1	1.0200	0.0000	138.0	1	2.0000	0.0000;
];

% generator data
% bus Pg Qg Qmax Qmin Vg mBase status Pmax Pmin
mpc.gen = [
	1	-9.4017	-63.5331	0.0000	0.0000	1.0000	100.0	1	1000000000.0000	-1000000000.0000;
	3	50.0000	85.3280	100.0000	-100.0000	1.0200	100.0	1	1000000000.0000	-1000000000.0000;
];

% branch data
% fbus tbus r x b rateA rateB rateC ratio angle status angmin angmax
mpc.branch = [
	1	2	0.020000	0.060000	0.000000	100.0	0.0	0.0	1.0000	0.0000	1	-360.0	360.0;
	2	3	0.010000	0.030000	0.000000	100.0	0.0	0.0	1.0000	0.0000	1	-360.0	360.0;
	1	3	0.012000	0.036000	0.000000	100.0	0.0	0.0	1.0000	0.0000	1	-360.0	360.0;
];
