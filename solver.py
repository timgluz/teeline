#!/usr/bin/python
# -*- coding: utf-8 -*-

"""Simple travelling salesman problem on a circuit board."""

from __future__ import print_function
import sys
import math
import os
from subprocess import Popen, PIPE
from ortools.constraint_solver import routing_enums_pb2
from ortools.constraint_solver import pywrapcp

def create_data_model(cities):
    """Stores the data for the problem."""
    data = {}
    data['locations'] = cities
    # Locations in block units
    data['locations2'] = [

    ]  # yapf: disable
    data['num_vehicles'] = 1
    data['depot'] = 0
    return data
    # [END data_model]


def compute_euclidean_distance_matrix(locations):
    """Creates callback to return distance between points."""
    distances = {}
    for from_counter, from_node in enumerate(locations):
        distances[from_counter] = {}
        for to_counter, to_node in enumerate(locations):
            if from_counter == to_counter:
                distances[from_counter][to_counter] = 0
            else:
                # Euclidean distance
                distances[from_counter][to_counter] = (int(
                    math.hypot((from_node[0] - to_node[0]),
                               (from_node[1] - to_node[1]))))
    return distances


def print_solution(manager, routing, solution):
    """Prints solution on console."""
    index = routing.Start(0)

    route = ''
    route_distance = 0
    while not routing.IsEnd(index):
        route += '{} '.format(manager.IndexToNode(index))
        previous_index = index
        index = solution.Value(routing.NextVar(index))
        route_distance += routing.GetArcCostForVehicle(previous_index, index, 0)

    route_distance += routing.GetArcCostForVehicle(previous_index, 0, 0)

    plan_output = '{} 1\n'.format(route_distance);
    plan_output += route;

    return plan_output

def main(cities):
    """Entry point of the program."""
    # Instantiate the data problem.
    n_cities = len(cities)
    data = create_data_model(cities)

    if n_cities > 1000:
        solver = 'greedy'
    else:
        solver = 'local_search'

    # Create the routing index manager.
    manager = pywrapcp.RoutingIndexManager(len(data['locations']),
                                           data['num_vehicles'], data['depot'])


    # Create Routing Model.
    routing = pywrapcp.RoutingModel(manager)
    distance_matrix = compute_euclidean_distance_matrix(data['locations'])

    def distance_callback(from_index, to_index):
        """Returns the distance between the two nodes."""
        # Convert from routing variable Index to distance matrix NodeIndex.
        from_node = manager.IndexToNode(from_index)
        to_node = manager.IndexToNode(to_index)
        return distance_matrix[from_node][to_node]

    transit_callback_index = routing.RegisterTransitCallback(distance_callback)

    # Define cost of each arc.
    routing.SetArcCostEvaluatorOfAllVehicles(transit_callback_index)


    # Setting first solution heuristic.
    search_parameters = pywrapcp.DefaultRoutingSearchParameters()

    search_parameters.first_solution_strategy = (
        routing_enums_pb2.FirstSolutionStrategy.PATH_CHEAPEST_ARC)


    if solver == 'simulated_annealing':
        search_parameters.local_search_metaheuristic = (
            routing_enums_pb2.LocalSearchMetaheuristic.SIMULATED_ANNEALING)
    elif solver == 'tabu_search':
        search_parameters.local_search_metaheuristic = (
            routing_enums_pb2.LocalSearchMetaheuristic.TABU_SEARCH)
    elif solver == 'greedy_descent':
        search_parameters.local_search_metaheuristic = (
            routing_enums_pb2.LocalSearchMetaheuristic.GREEDY_DESCENT)
    else:
        search_parameters.local_search_metaheuristic = (
            routing_enums_pb2.LocalSearchMetaheuristic.GUIDED_LOCAL_SEARCH)

    search_parameters.time_limit.seconds = 120

    # Solve the problem.
    solution = routing.SolveWithParameters(search_parameters)

    # Print solution on console.
    if solution:
        output_data = print_solution(manager, routing, solution)
        return output_data

def read_input_size(input_data):
    lines = input_data.split('\n')
    node_count = int(lines[0])
    return node_count

def parse_cities(input_data):
    # parse the input
    lines = input_data.split('\n')
    node_count = int(lines[0])
    points = []
    for i in range(1, node_count+1):
        line = lines[i]
        parts = line.split()
        points.append((float(parts[0]), float(parts[1])))
    return points

def solve_with_or_tools(input_data):
    # Modify this code to run your optimization algorithm
    cities = parse_cities(input_data)
    output_data = main(cities)

    return output_data


def solve_with_2opt(input_data):
    # Writes the inputData to a temporay file
    tmp_file_name = 'tmp.data'
    tmp_file = open(tmp_file_name, 'w')
    tmp_file.write(input_data)
    tmp_file.close()

    process = Popen(['cat ' + tmp_file_name + ' | ./solution'],
        stdout=PIPE, shell = True, universal_newlines = True
    )
    (stdout, stderr) = process.communicate()

    # removes the temporary file
    os.remove(tmp_file_name)

    return stdout.strip()

def solve_it(input_data):
    # switch solver based on the input size
    solution = 'None'

    if read_input_size(input_data) > 2000:
        solution = solve_with_2opt(input_data)
    else:
        solution = solve_with_or_tools(input_data)

    return solution


if __name__ == '__main__':
    import sys
    if len(sys.argv) > 1:
        file_location = sys.argv[1].strip()
        with open(file_location, 'r') as input_data_file:
            input_data = input_data_file.read()

        print(solve_it(input_data))
    else:
        print('This test requires an input file.  Please select one from the data directory. (i.e. python solver.py ./data/tsp_51_1)')

