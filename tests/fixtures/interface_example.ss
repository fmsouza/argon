// SafeScript interface and generics example

interface Drawable {
    draw(canvas: &mut Canvas): void;
    getX(): f64 with & this;
    getY(): f64 with & this;
}

class Canvas {
    private width: u32;
    private height: u32;
    private pixels: Vec<u8>;
    
    constructor(width: u32, height: u32) {
        this.width = width;
        this.height = height;
        this.pixels = new Vec(width * height * 4);
    }
    
    drawPixel(x: u32, y: u32, r: u8, g: u8, b: u8): void with &mut this {
        const idx = (y * this.width + x) * 4;
        this.pixels[idx] = r;
        this.pixels[idx + 1] = g;
        this.pixels[idx + 2] = b;
        this.pixels[idx + 3] = 255;
    }
}

class Circle implements Drawable {
    private x: f64;
    private y: f64;
    private radius: f64;
    private color: string;
    
    constructor(x: f64, y: f64, radius: f64, color: string) {
        this.x = x;
        this.y = y;
        this.radius = radius;
        this.color = color;
    }
    
    draw(canvas: &mut Canvas): void with &mut this {
        // Simplified - just draw center point
        canvas.drawPixel(
            this.x as u32,
            this.y as u32,
            255, 0, 0
        );
    }
    
    getX(): f64 with & this {
        return this.x;
    }
    
    getY(): f64 with & this {
        return this.y;
    }
}

class Rectangle implements Drawable {
    private x: f64;
    private y: f64;
    private width: f64;
    private height: f64;
    
    constructor(x: f64, y: f64, width: f64, height: f64) {
        this.x = x;
        this.y = y;
        this.width = width;
        this.height = height;
    }
    
    draw(canvas: &mut Canvas): void with &mut this {
        canvas.drawPixel(
            this.x as u32,
            this.y as u32,
            0, 255, 0
        );
    }
    
    getX(): f64 with & this {
        return this.x;
    }
    
    getY(): f64 with & this {
        return this.y;
    }
}

function renderAll<T extends Drawable>(items: &[T], canvas: &mut Canvas): void with &mut {
    for (const item of items) {
        item.draw(canvas);
    }
}

function main(): void {
    const canvas = new Canvas(800, 600);
    
    const shapes: Vec<Drawable> = new Vec();
    shapes.push(new Circle(100.0, 100.0, 50.0, "red"));
    shapes.push(new Rectangle(200.0, 200.0, 100.0, 50.0));
    
    console.log("Rendering shapes...");
    // renderAll(&shapes, &mut canvas);
}

main();
