-- Northwind sample database for MySQL
-- This is a simplified version of the classic Northwind database

USE northwind;

-- Customers table
CREATE TABLE IF NOT EXISTS customers (
    customer_id VARCHAR(5) PRIMARY KEY,
    company_name VARCHAR(100) NOT NULL,
    contact_name VARCHAR(50),
    contact_title VARCHAR(50),
    address VARCHAR(200),
    city VARCHAR(50),
    region VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(50),
    phone VARCHAR(30),
    fax VARCHAR(30)
);

-- Employees table
CREATE TABLE IF NOT EXISTS employees (
    employee_id INT AUTO_INCREMENT PRIMARY KEY,
    last_name VARCHAR(50) NOT NULL,
    first_name VARCHAR(50) NOT NULL,
    title VARCHAR(100),
    title_of_courtesy VARCHAR(30),
    birth_date DATE,
    hire_date DATE,
    address VARCHAR(200),
    city VARCHAR(50),
    region VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(50),
    home_phone VARCHAR(30),
    extension VARCHAR(10),
    photo VARCHAR(255),
    notes TEXT,
    reports_to INT,
    photo_path VARCHAR(255),
    FOREIGN KEY (reports_to) REFERENCES employees(employee_id)
);

-- Categories table
CREATE TABLE IF NOT EXISTS categories (
    category_id INT AUTO_INCREMENT PRIMARY KEY,
    category_name VARCHAR(50) NOT NULL,
    description TEXT,
    picture VARCHAR(255)
);

-- Products table
CREATE TABLE IF NOT EXISTS products (
    product_id INT AUTO_INCREMENT PRIMARY KEY,
    product_name VARCHAR(100) NOT NULL,
    supplier_id INT,
    category_id INT,
    quantity_per_unit VARCHAR(50),
    unit_price DECIMAL(10, 2) DEFAULT 0,
    units_in_stock SMALLINT DEFAULT 0,
    units_on_order SMALLINT DEFAULT 0,
    reorder_level SMALLINT DEFAULT 0,
    discontinued BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (category_id) REFERENCES categories(category_id)
);

-- Shippers table
CREATE TABLE IF NOT EXISTS shippers (
    shipper_id INT AUTO_INCREMENT PRIMARY KEY,
    company_name VARCHAR(100) NOT NULL,
    phone VARCHAR(30)
);

-- Orders table
CREATE TABLE IF NOT EXISTS orders (
    order_id INT AUTO_INCREMENT PRIMARY KEY,
    customer_id VARCHAR(5),
    employee_id INT,
    order_date DATETIME,
    required_date DATETIME,
    shipped_date DATETIME,
    ship_via INT,
    freight DECIMAL(10, 2) DEFAULT 0,
    ship_name VARCHAR(100),
    ship_address VARCHAR(200),
    ship_city VARCHAR(50),
    ship_region VARCHAR(50),
    ship_postal_code VARCHAR(20),
    ship_country VARCHAR(50),
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id),
    FOREIGN KEY (employee_id) REFERENCES employees(employee_id),
    FOREIGN KEY (ship_via) REFERENCES shippers(shipper_id)
);

-- Order Details table
CREATE TABLE IF NOT EXISTS order_details (
    order_id INT,
    product_id INT,
    unit_price DECIMAL(10, 2) NOT NULL,
    quantity SMALLINT NOT NULL DEFAULT 1,
    discount DECIMAL(4, 2) NOT NULL DEFAULT 0,
    PRIMARY KEY (order_id, product_id),
    FOREIGN KEY (order_id) REFERENCES orders(order_id),
    FOREIGN KEY (product_id) REFERENCES products(product_id)
);

-- Insert sample data - Add all required customers first
INSERT INTO customers (customer_id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone) VALUES
('ALFKI', 'Alfreds Futterkiste', 'Maria Anders', 'Sales Representative', 'Obere Str. 57', 'Berlin', NULL, '12209', 'Germany', '030-0074321'),
('ANATR', 'Ana Trujillo Emparedados y helados', 'Ana Trujillo', 'Owner', 'Avda. de la Constitución 2222', 'México D.F.', NULL, '05021', 'Mexico', '(5) 555-4729'),
('ANTON', 'Antonio Moreno Taquería', 'Antonio Moreno', 'Owner', 'Mataderos  2312', 'México D.F.', NULL, '05023', 'Mexico', '(5) 555-3932'),
('BERGS', 'Berglunds snabbköp', 'Christina Berglund', 'Order Administrator', 'Berguvsvägen  8', 'Luleå', NULL, 'S-958 22', 'Sweden', '0921-12 34 65'),
('VINET', 'Vins et alcools Chevalier', 'Paul Henriot', 'Accounting Manager', '59 rue de l''Abbaye', 'Reims', NULL, '51100', 'France', '26.47.15.10'),
('TOMSP', 'Toms Spezialitäten', 'Karin Josephs', 'Marketing Manager', 'Luisenstr. 48', 'Münster', NULL, '44087', 'Germany', '0251-034258'),
('HANAR', 'Hanari Carnes', 'Mario Pontes', 'Accounting Manager', 'Rua do Paço, 67', 'Rio de Janeiro', 'RJ', '05454-876', 'Brazil', '(21) 555-0091'),
('VICTE', 'Victuailles en stock', 'Mary Saveley', 'Sales Agent', '2, rue du Commerce', 'Lyon', NULL, '69004', 'France', '78.32.54.86'),
('SUPRD', 'Suprêmes délices', 'Pascale Cartrain', 'Accounting Manager', 'Boulevard Tirou, 255', 'Charleroi', NULL, 'B-6000', 'Belgium', '(071) 23-67-28-99');

INSERT INTO employees (employee_id, last_name, first_name, title, birth_date, hire_date) VALUES
(1, 'Davolio', 'Nancy', 'Sales Representative', '1968-12-08', '1994-05-03'),
(2, 'Fuller', 'Andrew', 'Vice President, Sales', '1952-02-19', '1992-08-14'),
(3, 'Leverling', 'Janet', 'Sales Representative', '1963-08-30', '1992-04-01'),
(4, 'Peacock', 'Margaret', 'Sales Representative', '1958-09-19', '1993-05-03'),
(5, 'Buchanan', 'Steven', 'Sales Manager', '1955-03-04', '1993-10-17'),
(6, 'Suyama', 'Michael', 'Sales Representative', '1963-07-02', '1993-10-17');

INSERT INTO categories (category_id, category_name, description) VALUES
(1, 'Beverages', 'Soft drinks, coffees, teas, beers, and ales'),
(2, 'Condiments', 'Sweet and savory sauces, relishes, spreads, and seasonings'),
(3, 'Confections', 'Desserts, candies, and sweet breads'),
(4, 'Dairy Products', 'Cheeses'),
(5, 'Grains/Cereals', 'Breads, crackers, pasta, and cereal'),
(6, 'Meat/Poultry', 'Prepared meats'),
(7, 'Produce', 'Dried fruit and bean curd'),
(8, 'Seafood', 'Seaweed and fish');

INSERT INTO products (product_id, product_name, category_id, unit_price, units_in_stock) VALUES
(1, 'Chai', 1, 18.00, 39),
(2, 'Chang', 1, 19.00, 17),
(3, 'Aniseed Syrup', 2, 10.00, 13),
(4, 'Chef Anton''s Cajun Seasoning', 2, 22.00, 53),
(5, 'Chef Anton''s Gumbo Mix', 2, 21.35, 0),
(6, 'Grandma''s Boysenberry Spread', 2, 25.00, 120),
(7, 'Uncle Bob''s Organic Dried Pears', 7, 30.00, 15),
(8, 'Northwoods Cranberry Sauce', 2, 40.00, 6),
(9, 'Mishi Kobe Niku', 6, 97.00, 29),
(11, 'Queso Cabrales', 4, 21.00, 22),
(14, 'Tofu', 7, 23.25, 35),
(42, 'Singaporean Hokkien Fried Mee', 5, 14.00, 26),
(51, 'Manjimup Dried Apples', 7, 53.00, 20),
(72, 'Mozzarella di Giovanni', 4, 34.80, 14);

INSERT INTO shippers (shipper_id, company_name, phone) VALUES
(1, 'Speedy Express', '(503) 555-9831'),
(2, 'United Package', '(503) 555-3199'),
(3, 'Federal Shipping', '(503) 555-9931');

INSERT INTO orders (order_id, customer_id, employee_id, order_date, required_date, ship_via, freight) VALUES
(10248, 'VINET', 5, '1996-07-04', '1996-08-01', 3, 32.38),
(10249, 'TOMSP', 6, '1996-07-05', '1996-08-16', 1, 11.61),
(10250, 'HANAR', 4, '1996-07-08', '1996-08-05', 2, 65.83),
(10251, 'VICTE', 3, '1996-07-08', '1996-08-05', 1, 41.34),
(10252, 'SUPRD', 4, '1996-07-09', '1996-08-06', 2, 51.30);

INSERT INTO order_details (order_id, product_id, unit_price, quantity, discount) VALUES
(10248, 11, 14.00, 12, 0),
(10248, 42, 9.80, 10, 0),
(10248, 72, 34.80, 5, 0),
(10249, 14, 18.60, 9, 0),
(10249, 51, 42.40, 40, 0);

-- Create indexes for better performance
CREATE INDEX idx_orders_customer_id ON orders(customer_id);
CREATE INDEX idx_orders_employee_id ON orders(employee_id);
CREATE INDEX idx_order_details_product_id ON order_details(product_id);
CREATE INDEX idx_products_category_id ON products(category_id);
